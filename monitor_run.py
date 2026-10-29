import urllib.request
import json
import time

url = 'https://api.github.com/repos/Alexpix0/PKMON-RELOADED-ANDROID/actions/runs/40938481035/jobs'
req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})

for _ in range(8):
    try:
        with urllib.request.urlopen(req) as resp:
            data = json.loads(resp.read().decode())
            if data.get('jobs'):
                job = data['jobs'][0]
                status = job['status']
                conclusion = job.get('conclusion')
                print(f"Job status: {status}, conclusion: {conclusion}")
                for s in job.get('steps', []):
                    if s['status'] == 'in_progress' or s.get('conclusion') == 'failure':
                        print(f"  Current step: {s['name']} -> {s['status']} ({s.get('conclusion')})")
                if status == 'completed':
                    for s in job.get('steps', []):
                        print(f"  Final step: {s['name']} -> {s.get('conclusion')}")
                    break
    except Exception as e:
        print('Error:', e)
    time.sleep(5)
