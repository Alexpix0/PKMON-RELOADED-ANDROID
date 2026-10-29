import urllib.request
import json

url = 'https://api.github.com/repos/Alexpix0/PKMON-RELOADED-ANDROID/commits/e7c925c/check-runs'
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0', 'Accept': 'application/vnd.github.v3+json'})
try:
    with urllib.request.urlopen(req) as resp:
        data = json.loads(resp.read().decode())
        for cr in data.get('check_runs', []):
            print(f"Check run ID: {cr['id']}")
            ann_url = cr.get('annotations_url')
            if ann_url:
                req_ann = urllib.request.Request(ann_url, headers={'User-Agent': 'Mozilla/5.0', 'Accept': 'application/vnd.github.v3+json'})
                with urllib.request.urlopen(req_ann) as resp_ann:
                    anns = json.loads(resp_ann.read().decode())
                    for a in anns:
                        print(f"[{a.get('path')}:{a.get('start_line')}] {a.get('message')}")
except Exception as e:
    print('Error:', e)
