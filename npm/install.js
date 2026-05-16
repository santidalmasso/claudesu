"use strict";

const fs = require("fs");
const path = require("path");
const https = require("https");
const crypto = require("crypto");

const REPO = "santidalmasso/claudesu";
const pkg = require("./package.json");

function fail(msg) {
  console.error("\nclaudesu: install failed — " + msg + "\n");
  process.exit(1);
}

function isSourceCheckout() {
  return fs.existsSync(path.join(__dirname, "..", "Cargo.toml"));
}

function skipReason() {
  if (process.env.CSU_SKIP_DOWNLOAD) {
    return "CSU_SKIP_DOWNLOAD set — skipping binary download.";
  }
  if (isSourceCheckout()) {
    return "source checkout detected — skipping binary download.";
  }
  return null;
}

function resolveTarget() {
  const { platform, arch } = process;
  if (platform === "darwin") {
    if (arch === "x64") return { triple: "x86_64-apple-darwin", exe: "" };
    if (arch === "arm64") return { triple: "aarch64-apple-darwin", exe: "" };
  } else if (platform === "linux") {
    if (arch === "x64") return { triple: "x86_64-unknown-linux-musl", exe: "" };
    if (arch === "arm64") return { triple: "aarch64-unknown-linux-musl", exe: "" };
  } else if (platform === "win32") {
    // Windows on ARM transparently emulates x64 binaries.
    return { triple: "x86_64-pc-windows-msvc", exe: ".exe" };
  }
  fail("unsupported platform/arch: " + platform + "/" + arch);
}

function assetName(target) {
  return "csu-" + target.triple + target.exe;
}

function releaseBaseUrl(version) {
  return "https://github.com/" + REPO + "/releases/download/v" + version;
}

function parseChecksum(text) {
  return text.trim().split(/\s+/)[0];
}

function sha256(buffer) {
  return crypto.createHash("sha256").update(buffer).digest("hex");
}

function download(url, redirects) {
  redirects = redirects || 0;
  return new Promise((resolve, reject) => {
    if (redirects > 10) {
      reject(new Error("too many redirects"));
      return;
    }
    https
      .get(url, { headers: { "User-Agent": "claudesu-npm-installer" } }, (res) => {
        const status = res.statusCode || 0;
        if (status >= 300 && status < 400 && res.headers.location) {
          res.resume();
          resolve(download(res.headers.location, redirects + 1));
          return;
        }
        if (status !== 200) {
          res.resume();
          reject(new Error("HTTP " + status + " for " + url));
          return;
        }
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function fetchAsset(base, asset) {
  try {
    return await download(base + "/" + asset);
  } catch (e) {
    fail(
      "could not download " + asset + " — " + e.message +
        "\nSee https://github.com/" + REPO + "/releases for available builds."
    );
  }
}

async function fetchExpectedChecksum(base, asset) {
  try {
    const text = (await download(base + "/" + asset + ".sha256")).toString("utf8");
    return parseChecksum(text);
  } catch (e) {
    return null;
  }
}

async function verifyChecksum(binary, base, asset) {
  const expected = await fetchExpectedChecksum(base, asset);
  if (!expected) {
    return; // checksum asset missing — verification is best effort
  }
  const actual = sha256(binary);
  if (expected !== actual) {
    fail(
      "checksum mismatch for " + asset +
        " (expected " + expected + ", got " + actual + ")"
    );
  }
}

function installBinary(binary, exe) {
  const binDir = path.join(__dirname, "bin");
  fs.mkdirSync(binDir, { recursive: true });
  const dest = path.join(binDir, "csu" + exe);
  fs.writeFileSync(dest, binary, { mode: 0o755 });
  console.log("claudesu: installed " + dest);
}

async function main() {
  const skip = skipReason();
  if (skip) {
    console.log("claudesu: " + skip);
    return;
  }

  const target = resolveTarget();
  const asset = assetName(target);
  const base = releaseBaseUrl(pkg.version);

  console.log("claudesu: downloading " + asset + " (v" + pkg.version + ")...");
  const binary = await fetchAsset(base, asset);
  await verifyChecksum(binary, base, asset);
  installBinary(binary, target.exe);
}

main().catch((e) => fail(e.message));
