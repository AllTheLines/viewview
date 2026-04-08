import json

file = "website/src/map_vector.styles.json"

with open(file, "r") as f:
    data = json.load(f)

for layer in data.get("layers", []):
    if "layout" not in layer:
        layer["layout"] = {}
    layer["layout"]["visibility"] = "none"

with open(file, "w") as f:
    json.dump(data, f, indent=2)
