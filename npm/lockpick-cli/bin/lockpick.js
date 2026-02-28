#!/usr/bin/env node

const { execFileSync } = require("child_process");
const { join } = require("path");

const PLATFORMS = {
  "darwin arm64": "lockpick-cli-darwin-arm64",
  "darwin x64": "lockpick-cli-darwin-x64",
  "linux x64": ["lockpick-cli-linux-x64-gnu", "lockpick-cli-linux-x64-musl"],
  "linux arm64": "lockpick-cli-linux-arm64-gnu",
  "win32 x64": "lockpick-cli-win32-x64",
};

function getBinaryPath() {
  const key = `${process.platform} ${process.arch}`;
  const candidates = PLATFORMS[key];

  if (!candidates) {
    throw new Error(
      `Unsupported platform: ${process.platform} ${process.arch}\n` +
        `lockpick-cli supports: darwin arm64/x64, linux x64/arm64, win32 x64`
    );
  }

  const names = Array.isArray(candidates) ? candidates : [candidates];
  const bin = process.platform === "win32" ? "lockpick.exe" : "lockpick";

  for (const name of names) {
    try {
      const binPath = require.resolve(join(name, bin));
      return binPath;
    } catch {}
  }

  throw new Error(
    `Could not find lockpick binary for ${process.platform} ${process.arch}.\n` +
      `Tried: ${names.join(", ")}\n` +
      `Make sure the platform package is installed. Try reinstalling lockpick-cli.`
  );
}

try {
  execFileSync(getBinaryPath(), process.argv.slice(2), {
    stdio: "inherit",
    env: process.env,
  });
} catch (e) {
  if (e.status !== undefined) {
    process.exit(e.status);
  }
  console.error(e.message);
  process.exit(1);
}
