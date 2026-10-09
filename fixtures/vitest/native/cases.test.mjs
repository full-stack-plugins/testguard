import {test,expect} from '/workspace/guard-toolchain/vitest-4.0.18/node_modules/vitest/dist/index.js';
import fs from 'node:fs';
const scenario=process.env.TESTGUARD_SCENARIO;
if(['pass','partial','missing-report','malformed'].includes(scenario)) {
 test('first case',()=>expect(1+1).toBe(2));
 if(scenario==='partial')test('second case',()=>expect(true).toBe(true));
} else if(scenario==='fail')test('failing assertion',()=>expect(1).toBe(2));
else if(scenario==='static-skip')test.skip('static skip',()=>expect(true).toBe(true));
else if(scenario==='skip')test('explicit skip',ctx=>ctx.skip());
else if(['timeout','hard-timeout'].includes(scenario)) {
 test('first case',()=>expect(1+1).toBe(2));
 test('hanging case',async()=>{
  fs.writeFileSync(process.env.TESTGUARD_STARTED,'started');
  await new Promise(()=>{});
 });
} else if(scenario==='collision') {
 for(const value of [1,2])test('duplicate parameter title',()=>expect(value).toBeGreaterThan(0));
}
