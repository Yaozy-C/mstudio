import { createContext, useContext, useState, type ReactNode } from "react";

type DisplayValue = boolean | number;
type DisplayState = Map<string, DisplayValue>;
export type ConversationStates = Map<string, DisplayState>;
const Context = createContext<DisplayState | null>(null);

export function ConversationState({
  id,
  states,
  children,
}: {
  id: string;
  states: ConversationStates;
  children: ReactNode;
}) {
  let state = states.get(id);
  if (!state) {
    state = new Map();
    states.set(id, state);
  }
  return <Context.Provider value={state}>{children}</Context.Provider>;
}

export function useConversationState<Value extends DisplayValue>(
  key: string,
  initial: Value,
) {
  const state = useContext(Context);
  const [value, setValue] = useState(
    () => (state?.get(key) as Value) ?? initial,
  );
  return [
    value,
    (next: Value) => {
      state?.set(key, next);
      setValue(next);
    },
  ] as const;
}
