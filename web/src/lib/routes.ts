export type RouteKey = "home" | "about" | "how";

export function parseHashRoute(hash: string): RouteKey {
  const raw = hash.trim();
  if (raw.length === 0) {
    return "home";
  }

  const cleaned = raw.startsWith("#") ? raw.slice(1) : raw;
  const pathname = cleaned.startsWith("/") ? cleaned.slice(1) : cleaned;

  if (pathname === "" || pathname === "/") {
    return "home";
  }

  const firstSegment = pathname.split("/")[0]?.toLowerCase() ?? "";
  if (firstSegment === "about") {
    return "about";
  }
  if (firstSegment === "how" || firstSegment === "how-it-works") {
    return "how";
  }
  return "home";
}
