# dummy-service

Three dummy Windows services for WinServX testing: `brequet-service-1` .. `brequet-service-3`.

Each service simulates a **slow start and a slow stop**: it reports `START_PENDING` / `STOP_PENDING` to the SCM for a random 1-10 s (updating checkpoints every 500 ms) before transitioning to `RUNNING` / `STOPPED`. Use them to verify that the WinServX UI shows start/stop transitions immediately without blocking.

## Build

```
cd dummy-services
cargo build --release
```

## Install (admin shell required)

```
.\target\release\dummy-service.exe install
```

## Use

Start/stop via WinServX, or from an admin shell:

```
sc start brequet-service-1
sc stop brequet-service-2
```

All three run from the same binary (`binPath = ... dummy-service.exe run <name>`).

## Log

Every start/stop phase (with the chosen delay) is appended to `%TEMP%\brequet-services.log`.

## Uninstall (admin shell required)

```
.\target\release\dummy-service.exe uninstall
```
