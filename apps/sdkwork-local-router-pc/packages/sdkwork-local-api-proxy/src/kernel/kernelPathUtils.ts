export function normalizeKernelPath(path?: string | null) {
  return (path || "").trim().replace(/\\/g, "/").replace(/\/+$/g, "");
}

export function joinKernelPath(...parts: Array<string | null | undefined>) {
  return normalizeKernelPath(
    parts
      .filter((part) => Boolean(part && String(part).trim()))
      .map((part) => String(part).trim().replace(/\\/g, "/").replace(/^\/+|\/+$/g, ""))
      .join("/"),
  );
}

export function dirnameKernelPath(path?: string | null) {
  const normalized = normalizeKernelPath(path);
  if (!normalized) {
    return "";
  }

  const lastSeparatorIndex = normalized.lastIndexOf("/");
  if (lastSeparatorIndex < 0) {
    return "";
  }
  if (lastSeparatorIndex === 0) {
    return "/";
  }

  return normalized.slice(0, lastSeparatorIndex);
}

export function isAbsoluteKernelPath(path?: string | null) {
  const normalized = normalizeKernelPath(path);
  return /^([a-zA-Z]:\/|\/|\/\/)/.test(normalized);
}
