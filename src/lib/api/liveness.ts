import { events } from '../tauri/bindings';
import type {
	ServiceConfigChanged,
	ServiceStatusChanged,
	ServicesChanged
} from '../tauri/bindings';

export interface LivenessHandlers {
	onStatusChanged: (event: ServiceStatusChanged) => void;
	onConfigChanged: (event: ServiceConfigChanged) => void;
	onServicesChanged: (event: ServicesChanged) => void;
}

export async function subscribeToLiveness(handlers: LivenessHandlers): Promise<Array<() => void>> {
	return Promise.all([
		events.serviceStatusChanged.listen((e) => handlers.onStatusChanged(e.payload)),
		events.serviceConfigChanged.listen((e) => handlers.onConfigChanged(e.payload)),
		events.servicesChanged.listen((e) => handlers.onServicesChanged(e.payload))
	]);
}
