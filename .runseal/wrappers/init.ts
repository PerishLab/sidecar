import { init } from "@perish/sealkit/init";

await init({
  tools: [
    "git",
    "deno",
    "cargo",
    "tea",
    "runseal",
    "sh",
    "ectropy",
    "plumb",
  ],
  paths: [
    "Cargo.toml",
    "Cargo.lock",
    "ectropy.toml",
    "runseal.toml",
    ".runseal/deno.json",
    ".runseal/deno.lock",
    ".runseal/wrappers/guard.ts",
    ".runseal/wrappers/init.ts",
    ".runseal/wrappers/land.ts",
    "plumb.toml",
    ".forgejo/workflows/guard.yml",
    ".forgejo/workflows/release-exact.yml",
    ".forgejo/workflows/release-stable.yml",
  ],
});
