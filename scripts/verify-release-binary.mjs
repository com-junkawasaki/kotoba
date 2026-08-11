#!/usr/bin/env node

import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";

const binary = resolve(process.argv[2] ?? "target/native/kotoba");
const work = mkdtempSync(join(tmpdir(), "kotoba-release-"));

function option(name) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] : undefined;
}

const release = {
  version: option("--release-version"),
  languageProfile: Number(option("--language-profile")),
  packageContract: Number(option("--package-contract")),
  commit: option("--commit"),
  tree: option("--tree"),
  platform: option("--platform"),
};

if (!/^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.test(release.version ?? "") ||
    release.languageProfile !== 6 || release.packageContract !== 1 ||
    !/^[0-9a-f]{40}$/.test(release.commit ?? "") ||
    !/^[0-9a-f]{40}$/.test(release.tree ?? "") ||
    !/^[a-z0-9]+-[a-z0-9]+$/.test(release.platform ?? "")) {
  throw new Error("release verifier requires the complete profile-6 release identity");
}

function run(args, expectedStatus = 0) {
  const result = spawnSync(binary, args, { encoding: "utf8" });
  if (result.error) throw result.error;
  if (result.status !== expectedStatus) {
    throw new Error(
      `${basename(binary)} ${args.join(" ")} exited ${result.status}\n${result.stdout}${result.stderr}`,
    );
  }
  return `${result.stdout}${result.stderr}`;
}

try {
  const binaryBytes = readFileSync(binary);
  const magic = binaryBytes.subarray(0, 4).toString("hex");
  const nativeMagic = new Set(["7f454c46", "feedfacf", "cffaedfe", "feedface", "cefaedfe"]);
  if (!nativeMagic.has(magic) && binaryBytes.subarray(0, 2).toString("ascii") !== "MZ") {
    throw new Error("release verifier requires a native executable, not a JVM or script launcher");
  }

  const extensionResults = {};
  for (const extension of ["kotoba", "cljk", "cljc"]) {
    const source = join(work, `portable.${extension}`);
    writeFileSync(source, "(defn main [] (+ 40 2))\n");
    const output = run(["run", source]);
    if (!/(^|\D)42(\D|$)/.test(output)) {
      throw new Error(`.${extension} did not evaluate the portable corpus to 42`);
    }
    extensionResults[extension] = "passed";
  }

  const safeIdentifierSource = join(work, "safe-window-name.kotoba");
  const safeIdentifierOutput = join(work, "safe-window-name.mjs");
  writeFileSync(
    safeIdentifierSource,
    "(ns timing (:export [shot-hit]))\n" +
      "(defn shot-hit [delta-present delta-ms window-ms]\n" +
      "  (if delta-present (if (<= delta-ms window-ms) 1 0) 0))\n",
  );
  run([
    "compile",
    safeIdentifierSource,
    "--target",
    "web",
    "--output",
    safeIdentifierOutput,
  ]);
  const safeIdentifierProbe = spawnSync(
    "node",
    [
      "--input-type=module",
      "-e",
      `import(${JSON.stringify(pathToFileURL(safeIdentifierOutput).href)}).then(m=>{const f=m.instantiateKotoba({})['shot-hit'];if(f(1n,150n,150n)!==1n||f(1n,151n,150n)!==0n||f(0n,0n,150n)!==0n)process.exit(2)})`,
    ],
    { encoding: "utf8" },
  );
  if (safeIdentifierProbe.error) throw safeIdentifierProbe.error;
  if (safeIdentifierProbe.status !== 0) {
    throw new Error(
      `safe ambient-name identifier probe failed: ${safeIdentifierProbe.stdout}${safeIdentifierProbe.stderr}`,
    );
  }

  const projectRoot = join(work, "project-src");
  const projectNamespace = join(projectRoot, "release", "probe");
  mkdirSync(projectNamespace, { recursive: true });
  const projectDependency = join(projectNamespace, "value.cljc");
  // The explicitly selected root deliberately lives beside project-src/;
  // namespace-discovered dependencies remain confined below projectRoot.
  const projectEntry = join(work, "project-main.cljc");
  const projectOutput = join(work, "project-probe.mjs");
  writeFileSync(
    projectDependency,
    "(ns release.probe.value \"bounded release project documentation\" (:export [answer]))\n" +
      "(defn answer [] 42)\n",
  );
  writeFileSync(
    projectEntry,
    "(ns release.probe.app\n" +
      "  (:require [release.probe.value :as value])\n" +
      "  (:export [main]))\n" +
      "(defn main [] (value/answer))\n",
  );
  run([
    "compile",
    projectEntry,
    "--source-path",
    projectRoot,
    "--unpinned",
    "--target",
    "web",
    "--output",
    projectOutput,
  ]);
  const projectProbe = spawnSync(
    "node",
    [
      "--input-type=module",
      "-e",
      `import(${JSON.stringify(pathToFileURL(projectOutput).href)}).then(m=>{if(m.instantiateKotoba({}).main()!==42n)process.exit(2)})`,
    ],
    { encoding: "utf8" },
  );
  if (projectProbe.error) throw projectProbe.error;
  if (projectProbe.status !== 0) {
    throw new Error(`closed .cljc project probe failed: ${projectProbe.stdout}${projectProbe.stderr}`);
  }

  const forbidden = join(work, "forbidden-eval.kotoba");
  writeFileSync(forbidden, "(defn main [] (eval '(+ 40 2)))\n");
  const rejectionText = run(["run", forbidden, "--json"], 1).trim();
  const rejection = JSON.parse(rejectionText);
  const diagnostic = rejection["kotoba.cli/diagnostic"] ?? rejection.diagnostic;
  if (diagnostic?.format !== "kotoba.diagnostic/v1" &&
      diagnostic?.format !== ":kotoba.diagnostic/v1") {
    throw new Error(`forbidden eval did not produce kotoba.diagnostic/v1: ${rejectionText}`);
  }
  if (rejectionText.includes(work)) {
    throw new Error("structured rejection disclosed its absolute source path");
  }

  const binarySha256 = createHash("sha256").update(binaryBytes).digest("hex");
  const evidence = {
    schema: "kotoba.release-evidence/v2",
    release,
    binary: basename(binary),
    binarySha256,
    runtime: "native-jvm-free",
    sourceExtensions: extensionResults,
    syntaxAwareAuthorityNames: "passed",
    closedCljcProjectSourcePath: "passed",
    structuredRejection: "passed",
  };
  const evidenceDirectory = resolve("target/native");
  mkdirSync(evidenceDirectory, { recursive: true });
  const evidencePath = join(evidenceDirectory, "release-evidence.json");
  writeFileSync(evidencePath, `${JSON.stringify(evidence, null, 2)}\n`);
  process.stdout.write(`${JSON.stringify(evidence)}\n`);
} finally {
  rmSync(work, { recursive: true, force: true });
}
