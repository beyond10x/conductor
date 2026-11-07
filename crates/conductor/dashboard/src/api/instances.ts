// What the header's instance selector reads (story:dashboard-instance-switch): `GET
// /api/instances`, every instance of the host registry with its own dashboard's page, the one this
// server serves marked `current`. crates/conductor/src/dashboard/instances.rs answers it, and
// crates/conductor/tests/dashboard_instances.rs holds its keys.

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

/** One instance of the host: `conductor.host.DashboardInstance`. */
export interface DashboardInstance {
  name: string;
  dashboard_port: number;
  /** Its dashboard's page, `http://127.0.0.1:<dashboard_port>/`. */
  url: string;
  /** Whether this server is that instance's dashboard. */
  current: boolean;
}

export interface Instances {
  instances: DashboardInstance[];
  errors: string[];
}

export async function loadInstances(
  fetcher: Fetch = (input, init) => fetch(input, init),
): Promise<{ instances: DashboardInstance[]; error: string | null }> {
  try {
    const answer = await fetcher('/api/instances', { cache: 'no-cache' });
    if (!answer.ok) {
      return { instances: [], error: `could not read /api/instances: ${answer.status}` };
    }
    const body = (await answer.json()) as Partial<Instances> | null;
    if (!Array.isArray(body?.instances)) {
      return { instances: [], error: 'could not read /api/instances: no instances' };
    }
    return { instances: body.instances, error: body.errors?.[0] ?? null };
  } catch (error) {
    return { instances: [], error: `could not read /api/instances: ${String(error)}` };
  }
}

/** The page of `instance` at the view `hash` names (`#/decisions`), so a switch keeps the view. */
export function pageOf(instance: DashboardInstance, hash: string): string {
  return `${instance.url}${hash}`;
}
