type UnauthorizedListener = () => void;

const unauthorizedListeners = new Set<UnauthorizedListener>();

export function emitUnauthorized(): void {
  unauthorizedListeners.forEach((listener) => {
    listener();
  });
}

export function subscribeToUnauthorized(
  listener: UnauthorizedListener
): () => void {
  unauthorizedListeners.add(listener);

  return () => {
    unauthorizedListeners.delete(listener);
  };
}
