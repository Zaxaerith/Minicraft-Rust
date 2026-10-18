#!/usr/bin/env python3
"""Compare regenerated official Java outputs with committed golden fixtures."""
from pathlib import Path
import json, zipfile
root=Path.cwd()
fixture=root/'crates/minicraft-client/tests/fixtures'
output=root/'target/parity'
checked=[]
def report(message):
    print(message, flush=True)
    checked.append(message)
def json_equal(a,b,label):
    left=json.loads(a.read_text(encoding='utf-8-sig'))
    right=json.loads(b.read_text(encoding='utf-8-sig'))
    assert left==right, label+' JSON content differs'
    report('PASS '+label)
def zip_equal(a,b,label):
    with zipfile.ZipFile(a) as left, zipfile.ZipFile(b) as right:
        ln=left.namelist(); rn=right.namelist()
        assert len(ln)==len(set(ln)) and len(rn)==len(set(rn)), label+' duplicate ZIP names'
        assert set(ln)==set(rn), label+' entry sets differ'
        for name in ln:
            assert left.read(name)==right.read(name), label+': bytes differ for '+name
        report('PASS '+label+': all '+str(len(ln))+' entry bytes')
        return len(ln)
names='beds boss-room36 colors entity-state35 flowers furniture generated-depths generated-flora migration-operations migration-registry migration-unlocks operations ornate-wood recipes sheep-natural36 sheep36 watering-flora watering-instances35 wool'.split()
assert len(names)==19
for name in names:
    json_equal(fixture/'java230'/f'{name}.json',output/'java230/output'/f'{name}.json','Java230 deterministic '+name+'.json')
for name in 'migration-base migration-colors migration-instances migration-entities35 migration-score60'.split():
    zip_equal(fixture/'java230'/f'{name}.zip',output/'java230/output'/f'{name}.zip','Java230 official Save '+name)
total=0
for size,seed in [(128,42),(128,256),(128,0),(128,-1),(256,42),(512,42)]:
    total+=zip_equal(fixture/'java230'/f'world36-{size}-{seed}.zip',output/'java230/world36'/f'{size}-{seed}.zip',f'World36 {size}/{seed}')
assert total==210
report('PASS World36 six worlds: all 210 packed entries')
normal=output/'java230/output/world35/normal-survival-42.zip'
zip_equal(fixture/'java230/normal-survival-42.zip',normal,'Uninstrumented normal Survival42 vs committed Phase35 fixture')
zip_equal(normal,output/'java230/world36/128-42/official-save.zip','Actual uninstrumented vs instrumented World.initWorld official Save')
json_equal(fixture/'java230/dungeon-chest-reward35.json',output/'java230/output/world35/dungeon-chest-reward35.json','Java230 final locked chest reward')
for name in ['generation.json','reference.json','tiles.json','advancements.json','Entities.miniplussave','Inventory.miniplussave','Player.miniplussave','dungeon-chests.txt']:
    left=fixture/'java224'/name; right=output/'java224/output'/name
    if name.endswith('.json'):
        json_equal(left,right,'Java224 fixture '+name)
    else:
        assert left.read_bytes()==right.read_bytes(), 'Java224 fixture '+name+' differs'
        report('PASS Java224 fixture '+name+': all bytes')
report('NOT COMPARED cross-JVM exact: normal-runtime-42 and unpersisted AI RNG; no fixture filtering or updates')
