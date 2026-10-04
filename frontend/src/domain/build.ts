import { operationContract } from "./operationContract";
const directory = process.argv[2];
if (!directory) throw new Error("Missing build output directory");
await Bun.write(
  `${directory}/operations.json`,
  JSON.stringify(operationContract),
);
const result = await Bun.build({
  entrypoints: [import.meta.dir + "/entry.ts"],
  outdir: directory,
  target: "browser",
  format: "iife",
  minify: true,
  naming: "domain.js",
});
if (!result.success)
  throw new AggregateError(result.logs, "Domain build failed");
