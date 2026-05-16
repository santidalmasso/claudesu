#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { spawnSync } = require("child_process");

const exe = process.platform === "win32" ? ".exe" : "";
const binary = path.join(__dirname, "csu" + exe);

if (!fs.existsSync(binary)) {
  console.error("claudesu: native binary not found at " + binary);
  console.error(
    "The install step may have failed — reinstall with: npm install -g claudesu"
  );
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error("claudesu: failed to run csu — " + result.error.message);
  process.exit(1);
}
process.exit(result.status === null ? 1 : result.status);
