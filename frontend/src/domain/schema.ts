// These constructors are the single source for advertised and validated arguments.
// No implicit open objects: each operation owns exactly the fields it consumes.
export type Schema = Record<string, unknown>;
export const text = (maxLength = 24000): Schema => ({
  type: "string",
  maxLength,
});
export const number = (minimum?: number, maximum?: number): Schema => ({
  type: "number",
  minimum,
  maximum,
});
export const integer = (minimum?: number, maximum?: number): Schema => ({
  ...number(minimum, maximum),
  type: "integer",
});
export const choices = (...values: (string | null)[]): Schema => ({
  type: values.includes(null) ? ["string", "null"] : "string",
  enum: values,
});
export const array = (items: Schema, maxItems = 30, minItems = 0): Schema => ({
  type: "array",
  items,
  minItems,
  maxItems,
});
export const object = (
  properties: Record<string, Schema>,
  required: string[] = [],
): Schema => ({
  type: "object",
  properties,
  required,
  additionalProperties: false,
});
export const boolean: Schema = { type: "boolean" };
export const nullable = (schema: Schema): Schema => ({
  ...schema,
  type: [schema.type, "null"],
});
export const id = text(100);

export const described = (schema: Schema, description: string): Schema => ({
  ...schema,
  description,
});
