// Stub for @tauri-apps/api/core in node test environment
export const invoke = async (_cmd: string, _args?: unknown): Promise<unknown> => undefined;
export const convertFileSrc = (filePath: string, _protocol?: string): string => filePath;
export const listen = async (_event: string, _handler: unknown): Promise<() => void> => () => {};
