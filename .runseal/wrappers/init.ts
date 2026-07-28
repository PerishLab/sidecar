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
    "manage.sh",
    "manage.ps1",
    "runseal.toml",
    ".runseal/deno.json",
    ".runseal/deno.lock",
    ".runseal/wrappers/guard.ts",
    ".runseal/wrappers/init.ts",
    ".runseal/wrappers/land.ts",
    ".runseal/wrappers/release.ts",
    ".forgejo/workflows/guard.yml",
    ".forgejo/workflows/release-beta.yml",
    ".forgejo/workflows/release-stable.yml",
  ],
});
