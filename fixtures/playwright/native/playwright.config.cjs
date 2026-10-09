module.exports={
 testDir:'.',testMatch:'cases.spec.cjs',fullyParallel:false,workers:1,retries:0,
 timeout:process.env.TESTGUARD_SCENARIO==='timeout'?1500:30000,
 reporter:[['line'],['json',{outputFile:process.env.TESTGUARD_REPORT}]],
 outputDir:'test-results',
 projects:[{name:'system-chromium',use:{browserName:'chromium',headless:true,launchOptions:{executablePath:'/usr/bin/chromium'}}}],
};
