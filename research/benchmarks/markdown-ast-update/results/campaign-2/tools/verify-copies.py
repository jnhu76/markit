import hashlib, json, os, sys
base = "results/campaign-2"
inv = json.load(open(os.path.join(base, "manifests/raw-inventory-v1.json")))
copies = {"A(worktree)": base,
          "B(/dev/shm)": "/dev/shm/campaign-2-copy-b",
          "C(/home/jnhu/markit-campaign-2-raw)": "/home/jnhu/markit-campaign-2-raw"}
bad = 0
checked = 0
for name, root in copies.items():
    ok = 0
    for entry in inv["entries"]:
        path = os.path.join(root, entry["path"])
        if not os.path.exists(path):
            print(f"MISSING {name}: {entry['path']}"); bad += 1; continue
        h = hashlib.sha256()
        with open(path, "rb") as fh:
            for chunk in iter(lambda: fh.read(1 << 22), b""):
                h.update(chunk)
        checked += 1
        if h.hexdigest() == entry["sha256"] and os.path.getsize(path) == entry["bytes"]:
            ok += 1
        else:
            print(f"MISMATCH {name}: {entry['path']}"); bad += 1
    print(f"{name}: {ok}/{len(inv['entries'])} files byte-identical")
print(f"VERIFIED_FILES={checked} MISMATCHES={bad}")
print("SECOND_COPY_VERIFIED = " + ("YES" if bad == 0 else "NO"))
sys.exit(1 if bad else 0)
