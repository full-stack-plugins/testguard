const fs=require('node:fs');
const scenario=process.env.TESTGUARD_SCENARIO;
if(['pass','partial','missing-report','malformed'].includes(scenario)) {
 test('first case',()=>expect(1+1).toBe(2));
 if(scenario==='partial')test('second case',()=>expect(true).toBe(true));
} else if(scenario==='fail')test('failing assertion',()=>expect(1).toBe(2));
else if(scenario==='skip')test.skip('explicit skip',()=>expect(true).toBe(true));
else if(['timeout','hard-timeout'].includes(scenario)) {
 test('first case',()=>expect(1+1).toBe(2));
 test('hanging case',async()=>{
  fs.writeFileSync(process.env.TESTGUARD_STARTED,'started');
  await new Promise(()=>{});
 });
} else if(scenario==='collision') {
 for(const value of [1,2])test('duplicate parameter title',()=>expect(value).toBeGreaterThan(0));
}
