function normalizeBaseUrl(baseUrl: string): string {
  if (baseUrl === '' || baseUrl === './') {
    return '/';
  }

  const withLeadingSlash = baseUrl.startsWith('/') ? baseUrl : `/${baseUrl}`;
  return withLeadingSlash.endsWith('/') ? withLeadingSlash : `${withLeadingSlash}/`;
}

const baseUrl = normalizeBaseUrl(import.meta.env.BASE_URL);

export function publicAsset(path: string): string {
  const cleanPath = path.replace(/^\/+/, '');
  return `${baseUrl}${cleanPath}`;
}

export function publicDir(path: string): string {
  const cleanPath = path.replace(/^\/+/, '').replace(/\/?$/, '/');
  return `${baseUrl}${cleanPath}`;
}
