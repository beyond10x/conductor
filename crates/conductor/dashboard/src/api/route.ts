// Where the app's reads and writes go (story:instance-api, design architecture-next § 5.2, § 5.3).
// The app is served by conductord at `/` and deployed elsewhere against the daemon's TLS listener;
// it reads the daemon's global API only. Each view names its data by the instance route it reads
// (`/api/board`, `/api/config`, `/data.json`, ...); [`routed`] turns that name into the instance
// API's route through the daemon's proxy, `<base>/v1/instances/<instance>/<route>`:
// - `<base>` is the page's `<meta name="conductor-api" content="https://...">`, the daemon's URL a
//   deployment names; absent or empty, the page's own origin (the daemon serving it);
// - `<instance>` is the page's `?instance=`, else the first instance `GET /v1/instances` lists;
// - `/api/<route>` is `<route>`, `/api/config` is `settings` (written with PUT), `/data.json` is
//   `data.json`.
// Against a deployed copy the page holds a bearer (`sessionStorage`, `conductor-bearer`), sent as
// `Authorization: Bearer`; the daemon's own page holds the daemon's token instead.

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

/** The key the bearer is kept under in `sessionStorage`. */
export const BEARER_KEY = 'conductor-bearer';

/** The daemon's URL the page names, without a trailing slash; empty for the page's own origin. */
export function apiBase(doc: Document = document): string {
  const named = doc.querySelector('meta[name="conductor-api"]')?.getAttribute('content') ?? '';
  return named.trim().replace(/\/+$/, '');
}

/** The instance the page's `?instance=` names, if any. */
export function namedInstance(search: string = window.location.search): string | null {
  const name = new URLSearchParams(search).get('instance');
  return name === null || name === '' ? null : name;
}

/** Whether `path` is one of the instance routes the views read. */
export function isInstanceRoute(path: string): boolean {
  return path.startsWith('/api/') || path === '/data.json' || path.startsWith('/data.json?');
}

/** The instance API's route of the view's `path` through the daemon at `base`, for `instance`. */
export function routed(path: string, instance: string, base: string = apiBase()): string {
  const [route = path, query] = path.split(/\?(.*)/s, 2);
  let name = route === '/data.json' ? 'data.json' : route.replace(/^\/api\//, '');
  if (name === 'config') {
    name = 'settings';
  }
  const tail = query === undefined || query === '' ? '' : `?${query}`;
  return `${base}/v1/instances/${encodeURIComponent(instance)}/${name}${tail}`;
}

/** The method a view's write takes on the instance API: the settings are written with PUT. */
export function routedMethod(path: string, method: string | undefined): string | undefined {
  return path.split('?')[0] === '/api/config' && method?.toUpperCase() === 'POST' ? 'PUT' : method;
}

/** The daemon's instances, as `GET /v1/instances` lists them. */
export interface InstanceRow {
  name: string;
  state: string;
  placement?: string;
}

/** Headers with the page's bearer added, when it holds one. */
export function withBearer(init: RequestInit | undefined, bearer: string | null): RequestInit {
  if (bearer === null || bearer === '') {
    return init ?? {};
  }
  const headers = new Headers(init?.headers);
  headers.set('Authorization', `Bearer ${bearer}`);
  return { ...init, headers };
}

function storedBearer(): string | null {
  try {
    return window.sessionStorage.getItem(BEARER_KEY);
  } catch {
    return null;
  }
}

/** Reads `GET /v1/instances` from the daemon at `base`. */
export async function loadDaemonInstances(
  fetcher: Fetch,
  base: string = apiBase(),
): Promise<{ instances: InstanceRow[]; error: string | null }> {
  try {
    const answer = await fetcher(`${base}/v1/instances`, { cache: 'no-cache' });
    if (!answer.ok) {
      return { instances: [], error: `could not read /v1/instances: ${answer.status}` };
    }
    const body = (await answer.json()) as unknown;
    if (!Array.isArray(body)) {
      return { instances: [], error: 'could not read /v1/instances: no list' };
    }
    return { instances: body as InstanceRow[], error: null };
  } catch (error) {
    return { instances: [], error: `could not read /v1/instances: ${String(error)}` };
  }
}

/**
 * A `fetch` that sends the views' instance routes to the instance API through the daemon, with the
 * page's bearer; every other request goes as it is, with the bearer.
 */
export function routingFetch(
  inner: Fetch,
  base: string = apiBase(),
  named: string | null = namedInstance(),
  bearer: () => string | null = storedBearer,
): Fetch {
  let instance: Promise<string | null> | null = null;
  const send: Fetch = (input, init) => inner(input, withBearer(init, bearer()));
  const current = (): Promise<string | null> => {
    if (instance === null) {
      instance =
        named !== null
          ? Promise.resolve(named)
          : loadDaemonInstances(send, base).then(({ instances }) => instances[0]?.name ?? null);
    }
    return instance;
  };
  return async (input, init) => {
    if (!isInstanceRoute(input)) {
      return send(input, init);
    }
    const name = await current();
    if (name === null) {
      return new Response('the daemon lists no instance', { status: 404 });
    }
    return send(routed(input, name, base), { ...init, method: routedMethod(input, init?.method) });
  };
}

/**
 * Installs [`routingFetch`] as the page's `fetch`. A deployed copy (one whose page names the
 * daemon's URL) asks once for the bearer it reads with, and keeps it for the tab.
 */
export function install(win: Window & typeof globalThis = window): void {
  const base = apiBase(win.document);
  if (base !== '' && (storedBearer() ?? '') === '') {
    const given = win.prompt(`The bearer this dashboard reads ${base} with`) ?? '';
    if (given.trim() !== '') {
      win.sessionStorage.setItem(BEARER_KEY, given.trim());
    }
  }
  const inner = win.fetch.bind(win);
  const routing = routingFetch(inner, base);
  win.fetch = ((input: RequestInfo | URL, init?: RequestInit) =>
    typeof input === 'string' ? routing(input, init) : inner(input, init)) as typeof fetch;
}
