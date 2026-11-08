// What the header's instance selector reads (story:dashboard-instance-switch, story:instance-api):
// the daemon's `GET /v1/instances`, every instance of its config file with its state. The page
// shows one instance at a time, the one its `?instance=` names, else the first listed; choosing
// another opens the same page for it at the same view. crates/conductor/src/daemon/api.rs answers
// it.

export { loadDaemonInstances as loadInstances, namedInstance, type InstanceRow } from './route';

/** The page for `instance` at the view `hash` names (`#/decisions`), so a switch keeps the view. */
export function pageOf(instance: string, hash: string, search: string = window.location.search): string {
  const params = new URLSearchParams(search);
  params.set('instance', instance);
  return `?${params.toString()}${hash}`;
}
