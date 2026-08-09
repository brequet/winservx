use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use crate::domain::service::{ServiceConfig, ServiceInfo, ServiceRuntimeStatus};

use super::events::{ServiceConfigChanged, ServiceStatusChanged};

/// In-memory read model of the services shown in the UI, kept fresh by the
/// liveness pipeline. The frontend receives the full snapshot through
/// `get_services` and granular deltas through events.
#[derive(Debug, Default)]
pub struct ServiceCache {
    services: HashMap<String, ServiceInfo>,
}

/// Result of reconciling the cache against a status-only enumeration.
#[derive(Debug, Default)]
pub struct StatusReconcile {
    /// Services whose runtime status changed.
    pub changed: Vec<ServiceStatusChanged>,
    /// The service set differs from the cache (services added or removed);
    /// callers should re-enumerate the full database.
    pub needs_full_refresh: bool,
}

/// Result of reconciling the cache against a full snapshot.
#[derive(Debug, Default)]
pub struct FullChangeSet {
    pub status_changed: Vec<ServiceStatusChanged>,
    pub config_changed: Vec<ServiceConfigChanged>,
    pub added: Vec<ServiceInfo>,
    pub removed: Vec<String>,
}

impl ServiceCache {
    pub fn snapshot(&self) -> Vec<ServiceInfo> {
        let mut services: Vec<ServiceInfo> = self.services.values().cloned().collect();
        services.sort_unstable_by(|left, right| left.name.cmp(&right.name));
        services
    }

    pub fn is_empty(&self) -> bool {
        self.services.is_empty()
    }

    /// Diffs the cache against a status-only enumeration, updating runtime state.
    pub fn apply_states(&mut self, states: &[ServiceRuntimeStatus]) -> StatusReconcile {
        let mut reconcile = StatusReconcile::default();
        let mut seen: HashSet<&str> = HashSet::with_capacity(states.len());
        for state in states {
            seen.insert(&state.name);
            let Some(cached) = self.services.get_mut(&state.name) else {
                reconcile.needs_full_refresh = true;
                continue;
            };
            if cached.state != state.state || cached.pid != state.pid {
                cached.state = state.state;
                cached.pid = state.pid;
                reconcile.changed.push(ServiceStatusChanged {
                    name: state.name.clone(),
                    state: state.state,
                    pid: state.pid,
                });
            }
        }
        if seen.len() != self.services.len() {
            reconcile.needs_full_refresh = true;
        }
        reconcile
    }

    /// Replaces the cache with a full snapshot, reporting every change.
    pub fn apply_full_snapshot(&mut self, fresh: Vec<ServiceInfo>) -> FullChangeSet {
        let mut change_set = FullChangeSet::default();
        let mut seen: HashSet<String> = HashSet::with_capacity(fresh.len());

        for info in fresh {
            seen.insert(info.name.clone());
            match self.services.entry(info.name.clone()) {
                Entry::Occupied(mut occupied) => {
                    let cached = occupied.get_mut();
                    let status_changed = cached.state != info.state || cached.pid != info.pid;
                    // A `None` start type means the config query failed; keep the cached value.
                    let config_changed =
                        info.start_type.is_some() && cached.start_type != info.start_type;
                    if status_changed {
                        cached.state = info.state;
                        cached.pid = info.pid;
                        change_set.status_changed.push(ServiceStatusChanged {
                            name: info.name.clone(),
                            state: info.state,
                            pid: info.pid,
                        });
                    }
                    if config_changed {
                        cached.start_type = info.start_type;
                        change_set.config_changed.push(ServiceConfigChanged {
                            name: info.name.clone(),
                            display_name: info.display_name.clone(),
                            start_type: info.start_type.expect("config_changed implies Some"),
                        });
                    }
                    // Display name/kind drift is rare; take fresh values silently.
                    cached.display_name = info.display_name;
                    cached.kind = info.kind;
                    cached.binary_path = info.binary_path;
                }
                Entry::Vacant(vacant) => {
                    change_set.added.push(info.clone());
                    vacant.insert(info);
                }
            }
        }

        let stale: Vec<String> = self
            .services
            .keys()
            .filter(|name| !seen.contains(*name))
            .cloned()
            .collect();
        for name in &stale {
            self.services.remove(name);
        }
        change_set.removed = stale;
        change_set
    }

    /// Applies a single service's runtime status, from a status-change notification.
    pub fn apply_status(&mut self, status: ServiceRuntimeStatus) -> Option<ServiceStatusChanged> {
        let cached = self.services.get_mut(&status.name)?;
        if cached.state == status.state && cached.pid == status.pid {
            return None;
        }
        cached.state = status.state;
        cached.pid = status.pid;
        Some(ServiceStatusChanged {
            name: status.name,
            state: status.state,
            pid: status.pid,
        })
    }

    /// Applies a single service's configuration, from a property-change notification.
    pub fn apply_config(
        &mut self,
        name: &str,
        config: ServiceConfig,
    ) -> Option<ServiceConfigChanged> {
        let cached = self.services.get_mut(name)?;
        if cached.display_name == config.display_name
            && cached.start_type == Some(config.start_type)
            && cached.binary_path == config.binary_path
        {
            return None;
        }
        cached.display_name = config.display_name.clone();
        cached.start_type = Some(config.start_type);
        cached.binary_path = config.binary_path;
        Some(ServiceConfigChanged {
            name: name.to_owned(),
            display_name: config.display_name,
            start_type: config.start_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::service::{ServiceKind, ServiceStartType, ServiceState};

    fn service(
        name: &str,
        state: ServiceState,
        pid: Option<u32>,
        start_type: Option<ServiceStartType>,
    ) -> ServiceInfo {
        ServiceInfo {
            name: name.to_owned(),
            display_name: name.to_uppercase(),
            state,
            start_type,
            kind: ServiceKind::Win32OwnProcess,
            pid,
            binary_path: String::new(),
        }
    }

    fn status(name: &str, state: ServiceState, pid: Option<u32>) -> ServiceRuntimeStatus {
        ServiceRuntimeStatus {
            name: name.to_owned(),
            state,
            pid,
        }
    }

    fn seeded_cache() -> ServiceCache {
        let mut cache = ServiceCache::default();
        cache.apply_full_snapshot(vec![
            service(
                "a",
                ServiceState::Running,
                Some(1),
                Some(ServiceStartType::Automatic),
            ),
            service("b", ServiceState::Stopped, None, Some(ServiceStartType::Manual)),
        ]);
        cache
    }

    #[test]
    fn apply_states_reports_only_changed_services() {
        let mut cache = seeded_cache();

        let reconcile = cache.apply_states(&[
            status("a", ServiceState::Running, Some(1)),
            status("b", ServiceState::Running, Some(42)),
        ]);

        assert!(!reconcile.needs_full_refresh);
        assert_eq!(reconcile.changed.len(), 1);
        assert_eq!(reconcile.changed[0].name, "b");
        assert_eq!(reconcile.changed[0].state, ServiceState::Running);
        assert_eq!(reconcile.changed[0].pid, Some(42));
        let cached = cache.snapshot().into_iter().find(|s| s.name == "b").unwrap();
        assert_eq!(cached.pid, Some(42));
        assert_eq!(cached.state, ServiceState::Running);
    }

    #[test]
    fn apply_states_requests_full_refresh_when_service_set_differs() {
        let mut cache = seeded_cache();

        let added = cache.apply_states(&[
            status("a", ServiceState::Running, Some(1)),
            status("c", ServiceState::Stopped, None),
        ]);
        assert!(added.needs_full_refresh);
        assert!(added.changed.is_empty());

        let removed = cache.apply_states(&[]);
        assert!(removed.needs_full_refresh);
    }

    #[test]
    fn full_snapshot_detects_added_removed_status_and_config() {
        let mut cache = seeded_cache();

        let change_set = cache.apply_full_snapshot(vec![
            service("a", ServiceState::Stopped, None, Some(ServiceStartType::Disabled)),
            service("c", ServiceState::Running, Some(7), Some(ServiceStartType::Automatic)),
        ]);

        assert_eq!(change_set.removed, vec!["b"]);
        assert_eq!(change_set.added.len(), 1);
        assert_eq!(change_set.added[0].name, "c");
        assert_eq!(change_set.status_changed.len(), 1);
        assert_eq!(change_set.status_changed[0].name, "a");
        assert_eq!(change_set.config_changed.len(), 1);
        assert_eq!(
            change_set.config_changed[0].start_type,
            ServiceStartType::Disabled
        );
        assert_eq!(cache.snapshot().len(), 2);
    }

    #[test]
    fn full_snapshot_keeps_cached_start_type_when_query_failed() {
        let mut cache = seeded_cache();

        let change_set =
            cache.apply_full_snapshot(vec![service("a", ServiceState::Running, Some(1), None)]);

        assert!(change_set.config_changed.is_empty());
        let cached = cache.snapshot().into_iter().find(|s| s.name == "a").unwrap();
        assert_eq!(cached.start_type, Some(ServiceStartType::Automatic));
    }

    #[test]
    fn full_snapshot_fills_missing_start_type() {
        let mut cache = ServiceCache::default();
        cache.apply_full_snapshot(vec![service("a", ServiceState::Running, None, None)]);

        let change_set =
            cache.apply_full_snapshot(vec![service("a", ServiceState::Running, None, Some(
                ServiceStartType::Manual,
            ))]);

        assert_eq!(change_set.config_changed.len(), 1);
        let cached = cache.snapshot().into_iter().next().unwrap();
        assert_eq!(cached.start_type, Some(ServiceStartType::Manual));
    }

    #[test]
    fn apply_status_updates_only_on_change() {
        let mut cache = seeded_cache();

        assert!(
            cache
                .apply_status(status("a", ServiceState::Running, Some(1)))
                .is_none()
        );
        let event = cache
            .apply_status(status("a", ServiceState::Stopped, None))
            .unwrap();
        assert_eq!(event.state, ServiceState::Stopped);
        assert!(
            cache
                .apply_status(status("ghost", ServiceState::Running, None))
                .is_none()
        );
    }

    #[test]
    fn apply_config_updates_display_name_and_start_type() {
        let mut cache = seeded_cache();

        assert!(
            cache
                .apply_config(
                    "a",
                    ServiceConfig {
                        display_name: "A".to_owned(),
                        binary_path: String::new(),
                        start_type: ServiceStartType::Automatic,
                    },
                )
                .is_none()
        );

        let event = cache
            .apply_config(
                "a",
                ServiceConfig {
                    display_name: "Alpha".to_owned(),
                    binary_path: String::new(),
                    start_type: ServiceStartType::Automatic,
                },
            )
            .unwrap();
        assert_eq!(event.display_name, "Alpha");

        assert!(
            cache
                .apply_config(
                    "ghost",
                    ServiceConfig {
                        display_name: "x".to_owned(),
                        binary_path: String::new(),
                        start_type: ServiceStartType::Manual,
                    },
                )
                .is_none()
        );
    }
}
