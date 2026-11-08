// story:instance-api: the views' instance routes go to the instance API through the daemon's
// proxy, at the page's own origin or the daemon URL a deployed copy names, with its bearer.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { apiBase, routed, routedMethod, routingFetch } from '../src/api/route';

afterEach(() => {
  document.head.querySelectorAll('meta[name="conductor-api"]').forEach((meta) => meta.remove());
});

describe('route', () => {
  it('names each view route by the instance API route through the daemon', () => {
    expect(routed('/api/board', 'alpha', '')).toBe('/v1/instances/alpha/board');
    expect(routed('/api/series?series=prs_merged&days=7', 'alpha', '')).toBe(
      '/v1/instances/alpha/series?series=prs_merged&days=7',
    );
    expect(routed('/api/config', 'alpha', '')).toBe('/v1/instances/alpha/settings');
    expect(routed('/data.json', 'alpha', 'https://daemon.example-org.test')).toBe(
      'https://daemon.example-org.test/v1/instances/alpha/data.json',
    );
    expect(routedMethod('/api/config', 'POST')).toBe('PUT');
    expect(routedMethod('/api/answers', 'POST')).toBe('POST');
  });

  it('reads the daemon URL a deployed copy names', () => {
    expect(apiBase()).toBe('');
    const meta = document.createElement('meta');
    meta.setAttribute('name', 'conductor-api');
    meta.setAttribute('content', 'https://daemon.example-org.test/');
    document.head.appendChild(meta);
    expect(apiBase()).toBe('https://daemon.example-org.test');
  });

  it('picks the first instance the daemon lists and sends the bearer', async () => {
    const inner = vi.fn(async (input: string) =>
      input.endsWith('/v1/instances') ? Response.json([{ name: 'alpha', state: 'Running' }]) : Response.json({}),
    );
    const fetcher = routingFetch(inner, 'https://daemon.example-org.test', null, () => 'reader');
    await fetcher('/api/board');
    await fetcher('/api/kpis');
    const urls = inner.mock.calls.map((call) => call[0]);
    expect(urls).toEqual([
      'https://daemon.example-org.test/v1/instances',
      'https://daemon.example-org.test/v1/instances/alpha/board',
      'https://daemon.example-org.test/v1/instances/alpha/kpis',
    ]);
    const init = (inner.mock.calls[1] as unknown as [string, RequestInit])[1];
    expect(new Headers(init.headers).get('Authorization')).toBe('Bearer reader');
  });
});
