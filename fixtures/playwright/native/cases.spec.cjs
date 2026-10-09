const {test,expect}=require(process.env.TESTGUARD_PLAYWRIGHT_PACKAGE+'/test');
const fs=require('node:fs');
const scenario=process.env.TESTGUARD_SCENARIO;
async function realPage({page,browser}) {
 expect(browser.version()).toBe('151.0.7922.173');
 await page.setContent('<title>TestGuard local fixture</title><button>ok</button>');
 await expect(page).toHaveTitle('TestGuard local fixture');
 await page.getByRole('button',{name:'ok'}).click();
}
if (['pass','partial','missing-report','malformed'].includes(scenario)) {
 test('first case',realPage);
 if(scenario==='partial') test('second case',realPage);
} else if(scenario==='fail') {
 test('failing assertion',async({page,browser})=>{await realPage({page,browser});expect(1).toBe(2);});
} else if(scenario==='skip') {
 test.skip('explicit skip',realPage);
} else if(['timeout','hard-timeout'].includes(scenario)) {
 test('first case',realPage);
 test('hanging case',async({page,browser})=>{await realPage({page,browser});fs.writeFileSync(process.env.TESTGUARD_STARTED,'started');await new Promise(()=>{});});
} else if(scenario==='collision') {
 for(const p of [1,2]) test('duplicate parameter title',async()=>{expect(p).toBeGreaterThan(0);});
}
