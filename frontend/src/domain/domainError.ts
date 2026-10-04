export class DomainError extends Error {
  constructor(
    public readonly code: string,
    message: string,
    public readonly context: Record<string, string>,
  ) {
    super(message);
  }
}
