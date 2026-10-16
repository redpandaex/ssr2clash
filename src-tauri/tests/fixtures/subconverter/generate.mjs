import { createServer } from 'node:http';
import { createServer as createTcpServer } from 'node:net';
import { spawn } from 'node:child_process';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const tool = resolve(process.argv[2]);
const output = dirname(fileURLToPath(import.meta.url));
const encode = text => Buffer.from(text).toString('base64url');
const nodes = [
  ['hk.example.invalid', '香港 01'],
  ['us.example.invalid', '美国 01'],
];
const subscription = Buffer.from(nodes.map(([host, name]) =>
  `ssr://${encode(`${host}:8388:auth_sha1_v4:aes-256-cfb:tls1.2_ticket_auth:${encode('test-password')}/?remarks=${encode(name)}&group=${encode('测试')}`)}`
).join('\n')).toString('base64');
const origin = createServer((request, response) => response.end(subscription));
await new Promise(resolve => origin.listen(0, '127.0.0.1', resolve));
const probe = createTcpServer();
await new Promise(resolve => probe.listen(0, '127.0.0.1', resolve));
const port = probe.address().port;
await new Promise(resolve => probe.close(resolve));
const config = (await readFile(join(tool, 'pref.example.ini'), 'utf8'))
  .replace(/^listen=.*$/m, 'listen=127.0.0.1')
  .replace(/^port=.*$/m, `port=${port}`);
await writeFile(join(tool, 'pref.ini'), config);
const child = spawn(join(tool, 'subconverter.exe'), [], { cwd: tool, windowsHide: true, stdio: 'ignore' });
try {
  let ready = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    if (child.exitCode !== null) throw new Error(`upstream exited: ${child.exitCode}`);
    try {
      ready = (await fetch(`http://127.0.0.1:${port}/version`)).ok;
      if (ready) break;
    } catch {}
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  if (!ready) throw new Error('upstream did not become ready');
  const source = `http://127.0.0.1:${origin.address().port}/subscribe`;
  const response = await fetch(`http://127.0.0.1:${port}/sub?target=clashr&url=${encodeURIComponent(source)}`);
  const yaml = await response.text();
  if (!response.ok || !yaml.includes('proxy-groups:') || !yaml.includes('hk.example.invalid')) {
    throw new Error(`invalid upstream output (${response.status}): ${yaml.slice(0, 200)}`);
  }
  await mkdir(output, { recursive: true });
  await writeFile(join(output, 'upstream.yaml'), yaml);
  await writeFile(join(output, 'input.txt'), subscription);
  await writeFile(join(output, 'source.json'), JSON.stringify({
    repository: 'https://github.com/tindy2013/subconverter',
    tag: 'v0.9.0', commit: 'b9ad0c2ee2b959e2b10195e928bf8f30f55a6a78', target: 'clashr',
    asset: 'subconverter_win64.7z', synthetic: true,
  }, null, 2) + '\n');
  const ruleSection = yaml.split(/\r?\n/).slice(yaml.split(/\r?\n/).indexOf('rules:') + 1).filter(line => line.trim());
  if (ruleSection.some(line => !/^  - [^"']/.test(line))) throw new Error('Unexpected upstream rule scalar format');
  const rules = ruleSection.map(line => line.slice(4));
  const resources = resolve(output, '../../../resources/presets/default');
  await mkdir(resources, { recursive: true });
  await writeFile(join(resources, 'rules.json'), JSON.stringify(rules, null, 2) + '\n');
  await writeFile(join(resources, 'groups.txt'), await readFile(join(tool, 'snippets/groups.txt')));
  await writeFile(join(resources, 'rulesets.txt'), await readFile(join(tool, 'snippets/rulesets.txt')));
  await writeFile(join(resources, 'source.json'), JSON.stringify({
    repository: 'https://github.com/tindy2013/subconverter', tag: 'v0.9.0',
    commit: 'b9ad0c2ee2b959e2b10195e928bf8f30f55a6a78', target: 'clashr',
    rules: rules.length, source: 'Official release default configuration; normalized Clash rules generated with synthetic SSR nodes.',
    regenerate: 'node src-tauri/tests/fixtures/subconverter/generate.mjs <extracted official v0.9.0 directory>',
  }, null, 2) + '\n');
  process.stdout.write(JSON.stringify({ bytes: Buffer.byteLength(yaml), rules: rules.length, output: join(output, 'upstream.yaml') }));
} finally {
  child.kill();
  origin.closeAllConnections();
  await new Promise(resolve => origin.close(resolve));
}
