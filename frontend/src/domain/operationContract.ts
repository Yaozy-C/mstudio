import contract from "./operations.json";
import type { Schema } from "./schema";

/** Data contract read directly by Rust and frontend tooling. */
export const operationContract: Schema = contract;
