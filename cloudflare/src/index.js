// Cloudflare Worker: public HTTPS entry point.
// Static Dioxus web assets (wasm/js/css) are served from Workers Static Assets before this runs.
// Everything else — the SSR page "/", server functions "/api/*", and "/<number>" redirects —
// is proxied to the Railway backend. Redirects are passed through untouched (redirect: "manual").
export default {
  async fetch(request, env) {
    const incoming = new URL(request.url);
    const target = new URL(incoming.pathname + incoming.search, env.BACKEND_URL);

    const headers = new Headers(request.headers);
    headers.set("x-forwarded-host", incoming.host);
    headers.set("x-forwarded-proto", "https");

    return fetch(target, {
      method: request.method,
      headers,
      body: ["GET", "HEAD"].includes(request.method) ? undefined : request.body,
      redirect: "manual",
    });
  },
};
