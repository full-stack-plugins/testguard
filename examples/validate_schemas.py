#!/usr/bin/env python3
"""Validate generated Draft-7 shape contracts. Rust tests cover cross-reference semantics."""
import copy,json,pathlib
from jsonschema import Draft7Validator
root=pathlib.Path(__file__).resolve().parents[1]
validators={}
for file in (root/'schemas').glob('*.json'):
    schema=json.loads(file.read_text());Draft7Validator.check_schema(schema);validators[file.stem]=Draft7Validator(schema)
source={'schema_version':'testguard.local/v1','capability':'local-fixture','approval_ref':'fixture:approval','revision':'revision1','baseline_digest':'a'*64,'requirements':['R1'],'sources':[{'id':'AC1','requirement_id':'R1','kind':'acceptance'}],'environments':['linux'],'obligations':[{'id':'O1','source_ids':['AC1'],'test_id':'case1','environments':['linux']}]}
validator=validators['test-obligation'];validator.validate(source)
for field,value in [('schema_version','other'),('capability','production'),('unknown',True)]:
    bad=copy.deepcopy(source);bad[field]=value;assert list(validator.iter_errors(bad)),field
for field in ['approval_ref','sources','environments','obligations']:
    bad=copy.deepcopy(source);del bad[field];assert list(validator.iter_errors(bad)),field
bad=copy.deepcopy(source);bad['sources'][0]['approved']=True;assert list(validator.iter_errors(bad))
print('5 schemas checked; 1 accepted and 8 rejected obligation shape fixtures; cross-reference semantics covered by cargo tests')
