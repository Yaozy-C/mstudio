import { errorText, type ErrorCode } from "./catalog";
/** Use at failure sources; UI consumes stable codes, not exception text. */
export function failure(code: ErrorCode, details: string): Error {
  return new Error(errorText(details, code));
}
