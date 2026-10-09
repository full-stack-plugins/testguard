module.exports={testMatch:['**/cases.test.cjs'],testEnvironment:'node',maxWorkers:1,cache:false,transform:{},testTimeout:process.env.TESTGUARD_SCENARIO==='timeout'?500:30000};
