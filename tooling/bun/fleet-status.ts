#!/usr/bin/env bun
/**
 * Fleet and account status in one JSON document; never prints a secret or an e-mail address.
 *  - ssh: reachability, CPU count and RAM of each host (default fr1 and prd; `ssh -G` gives the address);
 *  - OVH API (signed requests, credentials from ~/.ovh.conf) and Hostinger API (HOSTINGER_API_TOKEN):
 *    VMs listed with their state, matched to the fleet hosts by IPv4;
 *  - gcloud: kind of the active account and the active project;
 *  - wrangler whoami: login state and number of accounts.
 *
 * Run: bun tooling/bun/fleet-status.ts [host ...]
 */
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { resolveWranglerCmd } from "./cloudflare-deploy.ts";

const hosts = Bun.argv.slice(2).length ? Bun.argv.slice(2) : ["fr1", "prd"];
const redact = (text: string) => text.replace(/[\w.+-]+@[\w-]+(\.[\w-]+)+/g, "<email>");

async function run(command: string[], timeoutMs = 20_000) {
  const proc = Bun.spawn(command, { stdout: "pipe", stderr: "pipe", stdin: "ignore" });
  const timer = setTimeout(() => proc.kill(), timeoutMs);
  const [stdout, stderr, code] = await Promise.all([new Response(proc.stdout).text(), new Response(proc.stderr).text(), proc.exited]);
  clearTimeout(timer);
  return { code, stdout: stdout.trim(), stderr: stderr.trim() };
}

async function sshHost(host: string) {
  const config = Object.fromEntries((await run(["ssh", "-G", host])).stdout.split(/\r?\n/).map((line) => line.split(/ (.*)/s).slice(0, 2)));
  const probe = await run(["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=8", host, "nproc; awk '/MemTotal/{print $2}' /proc/meminfo; awk '/MemAvailable/{print $2}' /proc/meminfo; hostname"]);
  const [cpus, memTotal, memAvailable] = probe.stdout.split("\n").map(Number);
  const reachable = probe.code === 0;
  return {
    host,
    address: config.hostname as string,
    reachable,
    cpus: reachable ? cpus : null,
    ram_gb: reachable ? Math.round(memTotal / 1024 / 1024) : null,
    ram_available_gb: reachable ? Math.round(memAvailable / 1024 / 1024) : null,
  };
}

function ovhCredentials() {
  const sections: Record<string, Record<string, string>> = {};
  let current = "";
  for (const line of readFileSync(join(homedir(), ".ovh.conf"), "utf8").split(/\r?\n/)) {
    const section = line.match(/^\[(.+)]$/);
    if (section) current = section[1];
    else if (line.includes("=")) (sections[current] ??= {})[line.split("=")[0].trim()] = line.slice(line.indexOf("=") + 1).trim();
  }
  const endpoint = sections.default?.endpoint ?? "ovh-eu";
  const base: Record<string, string> = {
    "ovh-eu": "https://eu.api.ovh.com/1.0",
    "ovh-ca": "https://ca.api.ovh.com/1.0",
    "ovh-us": "https://api.us.ovhcloud.com/1.0",
  };
  return { base: base[endpoint], endpoint, ...sections[endpoint] };
}

async function ovh() {
  const credentials = ovhCredentials();
  const skew = Number(await (await fetch(`${credentials.base}/auth/time`)).text()) - Math.floor(Date.now() / 1000);
  const call = async (path: string) => {
    const url = `${credentials.base}${path}`;
    const timestamp = String(Math.floor(Date.now() / 1000) + skew);
    const signature = "$1$" + createHash("sha1").update([credentials.application_secret, credentials.consumer_key, "GET", url, "", timestamp].join("+")).digest("hex");
    const response = await fetch(url, {
      headers: { "X-Ovh-Application": credentials.application_key, "X-Ovh-Consumer": credentials.consumer_key, "X-Ovh-Timestamp": timestamp, "X-Ovh-Signature": signature },
    });
    if (!response.ok) throw new Error(`OVH ${path}: HTTP ${response.status}`);
    return response.json();
  };
  const names = (await call("/vps")) as string[];
  return Promise.all(
    names.map(async (name) => {
      const [info, ips] = await Promise.all([call(`/vps/${name}`), call(`/vps/${name}/ips`)]);
      return { provider: "ovh", name, state: info.state as string, ipv4: (ips as string[]).filter((ip) => !ip.includes(":")) };
    }),
  );
}

async function hostinger() {
  const token = process.env.HOSTINGER_API_TOKEN;
  if (!token) throw new Error("HOSTINGER_API_TOKEN is not set");
  const response = await fetch("https://developers.hostinger.com/api/vps/v1/virtual-machines", { headers: { Authorization: `Bearer ${token}` } });
  if (!response.ok) throw new Error(`Hostinger: HTTP ${response.status}`);
  const machines = (await response.json()) as { id: number; hostname: string; state: string; ipv4?: { address: string }[] }[];
  return machines.map((machine) => ({ provider: "hostinger", name: `${machine.id} ${machine.hostname}`, state: machine.state, ipv4: (machine.ipv4 ?? []).map((ip) => ip.address) }));
}

async function gcloud() {
  const [account, project] = await Promise.all([run(["gcloud", "config", "get-value", "account"]), run(["gcloud", "config", "get-value", "project"])]);
  const value = account.stdout;
  return { account: !value ? "none" : value.endsWith(".gserviceaccount.com") ? "service-account" : "user", project: project.stdout || "unset" };
}

async function wrangler() {
  const { code, stdout, stderr } = await run([...resolveWranglerCmd(), "whoami"], 90_000);
  const text = redact(`${stdout}\n${stderr}`);
  const accounts = text.split("\n").filter((line) => /^\s*│\s*\S.*│\s*[0-9a-f]{32}\s*│/.test(line)).length;
  return { logged_in: code === 0 && /logged in/i.test(text) && !/not authenticated/i.test(text), accounts };
}

const settle = async <T>(promise: Promise<T>) => promise.then((value) => ({ ok: true as const, value }), (error: Error) => ({ ok: false as const, error: redact(String(error.message)) }));
const [ssh, ovhVms, hostingerVms, gcloudStatus, wranglerStatus] = await Promise.all([
  Promise.all(hosts.map(sshHost)),
  settle(ovh()),
  settle(hostinger()),
  settle(gcloud()),
  settle(wrangler()),
]);
const vms = [...(ovhVms.ok ? ovhVms.value : []), ...(hostingerVms.ok ? hostingerVms.value : [])];
const fleet = ssh.map((host) => {
  const match = vms.find((vm) => vm.ipv4.includes(host.address));
  return { ...host, vm: match ? { provider: match.provider, name: match.name, state: match.state } : null };
});
const report = {
  fleet,
  providers: { ovh: ovhVms.ok ? { vms: ovhVms.value.length } : { error: ovhVms.error }, hostinger: hostingerVms.ok ? { vms: hostingerVms.value.length } : { error: hostingerVms.error } },
  gcloud: gcloudStatus.ok ? gcloudStatus.value : { error: gcloudStatus.error },
  wrangler: wranglerStatus.ok ? wranglerStatus.value : { error: wranglerStatus.error },
};
console.log(JSON.stringify(report, null, 2));
if (fleet.some((host) => !host.reachable || !host.vm)) process.exit(1);
