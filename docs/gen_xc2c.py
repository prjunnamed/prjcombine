import json

with open("../databases/xc2c.json") as f:
    db = json.load(f)

with open("xc2c/gen-devices.inc", "w") as f:
    f.write(".. list-table::\n")
    f.write("   :header-rows: 1\n")
    f.write("\n")
    f.write("   - - Device\n")
    f.write("     - IDCODE\n")
    f.write("     - Function Blocks\n")
    f.write("     - I/O banks\n")
    f.write("     - input pads\n")
    f.write("     - VREF\n")
    f.write("     - data gate\n")
    f.write("     - clock divider\n")
    for part in db["parts"]:
        device = db["devices"][part["device"]]
        f.write(f"   - - {part['name']}\n")
        f.write(f"     - ``0xX{device['idcode_part']:04x}093``\n")
        f.write(f"     - {device['fb_rows'] * len(device['fb_cols']) * 2}\n")
        f.write(f"     - {device['banks']}\n")
        f.write(f"     - {device['ipads']}\n")
        if device["has_vref"]:
            f.write(f"     - X\n")
        else:
            f.write(f"     - \\-\n")
        if "DGE" in device["io_special"]:
            f.write(f"     - X\n")
        else:
            f.write(f"     - \\-\n")
        if "CDR" in device["io_special"]:
            f.write(f"     - X\n")
        else:
            f.write(f"     - \\-\n")

packages = []
for part in db["parts"]:
    for pkg in part["packages"]:
        if pkg not in packages:
            packages.append(pkg)

with open("xc2c/gen-devices-pkg.inc", "w") as f:
    f.write(".. list-table::\n")
    f.write("   :header-rows: 1\n")
    f.write("\n")
    f.write("   - - Device\n")
    for pkg in packages:
        if not pkg.startswith("di"):
            f.write(f"     - {pkg}\n")
    f.write("     - Bare die\n")
    for part in db["parts"]:
        f.write(f"   - - {part['name']}\n")
        bare = None
        for pkg in packages:
            if pkg.startswith("di"):
                if pkg in part["packages"]:
                    bare = pkg
            else:
                if pkg in part["packages"]:
                    f.write(f"     - X\n")
                else:
                    f.write(f"     - \\-\n")
        if bare is not None:
            f.write(f"     - {bare}\n")
        else:
            f.write(f"     - \\-\n")


def gen_tile_items(f, tname, tile):
    for name, item in sorted(tile.items(), key=lambda x: x[1]["bits"]):
        f.write(f"<table class=\"docutils align-default prjcombine-enum\" id=\"bits-{tname}-{name}\">\n")
        f.write(f"<tr><th>{name}</th>")
        for bit in reversed(item["bits"]):
            f.write(f"<th>{bit}</th>")
        f.write("</tr>\n")
        if "values" in item:
            for vname, val in sorted(item["values"].items(), key=lambda x: x[1][::-1]):
                f.write(f"<tr><td>{vname}</td>")
                for v in reversed(val):
                    f.write(f"<td>{int(v)}</td>")
                f.write(f"</tr>\n")
        else:
            if item["invert"]:
                f.write("<tr><td>Inverted</td>")
                inv = "~"
            else:
                f.write("<tr><td>Non-inverted</td>")
                inv = ""
            for i in reversed(range(len(item["bits"]))):
                f.write(f"<td>{inv}[{i}]</td>")
            f.write("</tr>\n")
        f.write("</table>\n")

def gen_tile(f, tname, tile):
    f.write("<table class=\"docutils align-default prjcombine-tile\">\n")
    rev = {}
    rows = set()
    columns = set()
    for item in tile.values():
        for bit in item["bits"]:
            rows.add(bit[0])
            columns.add(bit[1])
    for name, item in tile.items():
        for j, bit in enumerate(item["bits"]):
            key = tuple(bit)
            if key not in rev:
                rev[key] = []
            rev[key].append((name, item, None if len(item["bits"]) == 1 else j))
    f.write(f"<tr><th rowspan=\"2\">Row</th><th colspan=\"{len(columns)}\">Column</th></tr>")
    f.write("<tr>")
    for col in sorted(columns):
        f.write(f"<th>{col}</th>")
    f.write("</tr>\n")
    for row in sorted(rows):
        f.write(f"<tr><td>{row}</td>\n")
        for col in sorted(columns):
            f.write("<td>")
            crd = (row, col)
            if crd in rev:
                for name, item, bidx in rev[crd]:
                    title = name
                    if bidx is not None:
                        title = f"{name}[{bidx}]"
                    if item.get("invert", False):
                        title = f"~{title}"
                    f.write(f"<a href=\"#bits-{tname}-{name}\" title=\"{title}\">X</a>")
            else:
                f.write("-")
            f.write("</td>")
        f.write("</tr>\n")
    f.write("</table>\n")

    gen_tile_items(f, tname, tile)

def gen_jed(f, tname, tile, bits):
    f.write("<table class=\"docutils align-default prjcombine-tile\">\n")
    f.write("<tr><th>JED offset</th><th>Bit</th></tr>\n")
    for i, (name, idx) in enumerate(bits):
        if len(tile[name]["bits"]) == 1:
            bname = name
        else:
            bname = f"{name}[{idx}]"
        f.write(f"<tr><td>{i}</td><td><a href=\"#bits-{tname}-{name}\">{bname}</a></td></tr>\n")
    f.write("</table>\n")

with open("xc2c/db-devices.rst", "w") as f:
    f.write("Database — devices\n")
    f.write("##################\n")
    f.write("\n")
    f.write(".. toctree::\n")
    f.write("   :caption: Contents:\n")
    f.write("\n")
    devs_done = set()
    for part in db["parts"]:
        if part["device"] not in devs_done:
            devs_done.add(part["device"])
            f.write(f"   db-device-{part['name']}\n")

for i, device in enumerate(db["devices"]):
    parts = []
    bonds = {}
    for part in db["parts"]:
        if part["device"] == i:
            parts.append(part)
            for pkg, bond in part["packages"].items():
                if bond not in bonds:
                    pins = {}
                    for k, v in db["bonds"][bond]["pins"].items():
                        if v not in pins:
                            pins[v] = []
                        pins[v].append(k)
                    bonds[bond] = ([], pins)
                bonds[bond][0].append(f"{part['name']}-{pkg}")
    bonds = {
        k: bonds[k]
        for k in sorted(bonds)
    }
    dev_packages = [pkg for pkg in packages if any(pkg in part["packages"] for part in parts)]
    with open(f"xc2c/db-device-{parts[0]['name']}.rst", "w") as f:
        names = ", ".join(part["name"].upper() for part in parts)
        fbs = device['fb_rows'] * len(device['fb_cols']) * 2
        io_special_rev = {f"IOB_{v[0]}_{v[1]}": k for k, v in device["io_special"].items()}
        f.write(f"{names}\n")
        l = "#" * len(names)
        f.write(f"{l}\n")
        f.write(f"\n")
        f.write(f"IDCODE part: {device['idcode_part']:#06x}\n")
        f.write(f"\n")
        f.write(f"FB count: {fbs}\n")
        f.write(f"\n")
        f.write(f"I/O banks: {device['banks']}\n")
        f.write(f"\n")
        f.write(f"Input-only pads: {device['ipads']}\n")
        f.write(f"\n")
        f.write(f"Has VREF: {device['has_vref']}\n")
        f.write(f"\n")
        f.write(f"BS cols: {device['bs_cols']}\n")
        f.write(f"\n")
        f.write(f"IMUX width: {device['imux_width']}\n")
        f.write(f"\n")
        f.write(f"BS layout: {device['bs_layout']}\n")
        f.write(f"\n")
        f.write(f"FB rows: {device['fb_rows']}\n")
        f.write(f"\n")
        f.write(f"MC width: {device['mc_width']}\n")
        f.write(f"\n")
        f.write(f"FB rows: {device['fb_rows']}\n")
        f.write(f"\n")
        f.write(f".. list-table::\n")
        f.write(f"   :header-rows: 1\n")
        f.write(f"\n")
        f.write(f"   - - Column range\n")
        f.write(f"     - Bits\n")
        items = []
        for bit in device["xfer_cols"]:
            items.append((bit, 1, "transfer"))
        for i, fbc in enumerate(device["fb_cols"]):
            items.append((fbc, device["mc_width"], f"FB column {i} even MCs"))
            fbc += device["mc_width"]
            if device["bs_layout"] == "NARROW":
                items.append((fbc, 32, f"FB column {i} even PTs OR"))
                fbc += 32
                items.append((fbc, 112, f"FB column {i} even PTs AND"))
                fbc += 112
            else:
                items.append((fbc, 112, f"FB column {i} even PTs"))
                fbc += 112
            items.append((fbc, device["imux_width"] * 2, f"FB column {i} IMUX"))
            fbc += device["imux_width"] * 2
            if device["bs_layout"] == "NARROW":
                items.append((fbc, 112, f"FB column {i} odd PTs AND"))
                fbc += 112
                items.append((fbc, 32, f"FB column {i} odd PTs OR"))
                fbc += 32
            else:
                items.append((fbc, 112, f"FB column {i} odd PTs"))
                fbc += 112
            items.append((fbc, device["mc_width"], f"FB column {i} odd MCs"))
            fbc += device["mc_width"]
        items.sort()
        for bit, width, item in items:
            f.write(f"   - - {bit}..{bit+width}\n")
            f.write(f"     - {item}\n")
        f.write(f"\n")
        f.write(f"I/O pins\n")
        f.write(f"========\n")
        f.write(f"\n")
        f.write(f".. list-table::\n")
        f.write(f"   :header-rows: 1\n")
        f.write(f"\n")
        f.write(f"   - - Function\n")
        f.write(f"     - Bank\n")
        f.write(f"     - Pad distance\n")
        for names, _ in bonds.values():
            f.write(f"     - {', '.join(names)}\n")
        f.write(f"   - - IDCODE part\n")
        f.write(f"     -\n")
        f.write(f"     -\n")
        for bond in bonds:
            bond = db["bonds"][bond]
            f.write(f"     - {bond['idcode_part']:#06x}\n")
        for io, pdata in sorted(device["ios"].items(), key=lambda io: (int(io[0].split("_")[1]), int(io[0].split("_")[2])) if "_" in io[0] else (-1, 0)):
            if io in io_special_rev:
                spec = io_special_rev[io]
                f.write(f"   - - {io} ({spec})\n")
            else:
                f.write(f"   - - {io}\n")
            f.write(f"     - {pdata['bank']}\n")
            f.write(f"     - {pdata['pad_distance']}\n")
            for _, pins in bonds.values():
                if io in pins:
                    f.write(f"     - {pins[io][0]}\n")
                else:
                    f.write(f"     - \\-\n")
        for pad in ["TCK", "TMS", "TDI", "TDO"]:
            f.write(f"   - - {pad}\n")
            f.write(f"     - AUX\n")
            f.write(f"     -\n")
            for _, pins in bonds.values():
                f.write(f"     - {pins[pad][0]}\n")
        specs = ["GND", "VCCINT"]
        for bank in range(device["banks"]):
            specs.append(f"VCCIO{bank}")
        specs += ["VCCAUX", "NC"]
        for spec in specs:
            f.write(f"   - - {spec}\n")
            if spec.startswith("VCCIO"):
                f.write(f"     - {spec[5:]}\n")
            else:
                f.write(f"     - \\-\n")
            f.write(f"     -\n")
            for _, pins in bonds.values():
                if spec in pins:
                    pins = pins[spec]
                    f.write(f"     - {pins[0]}\n")
                    for pin in pins[1:]:
                        f.write(f"\n")
                        f.write(f"       {pin}\n")
                else:
                    f.write(f"     - \\-\n")
        f.write(f"\n")
        f.write(f"Speed data\n")
        f.write(f"==========\n")
        f.write(f"\n")
        f.write(f".. list-table::\n")
        f.write(f"   :header-rows: 1\n")
        f.write(f"\n")
        f.write(f"   - - Timing parameter\n")
        for part in parts:
            speeds = sorted(part["speeds"], key=lambda s: int(s[1:]))
            for speed in speeds:
                f.write(f"     - {part['name']}{speed}\n")
        for key in db["speeds"][0]["timing"]:
            f.write(f"   - - {key}\n")
            for part in parts:
                speeds = sorted(part["speeds"], key=lambda s: int(s[1:]))
                for speed in speeds:
                    val = db["speeds"][part["speeds"][speed]]["timing"][key]
                    f.write(f"     - {val}\n")
        f.write(f"\n")
        f.write(f"IMUX bits\n")
        f.write(f"=========\n")
        f.write(f"\n")
        f.write(f".. raw:: html\n")
        f.write(f"   :file: gen-tile-{parts[0]['name']}-imux.html\n")
        with open(f"xc2c/gen-tile-{parts[0]['name']}-imux.html", "w") as tf:
            gen_tile(tf, "imux", device["imux_bits"])
        f.write(f"\n")
        f.write(f"MC bits\n")
        f.write(f"=======\n")
        f.write(f"\n")
        f.write(f".. raw:: html\n")
        f.write(f"   :file: gen-tile-{parts[0]['name']}-mc.html\n")
        with open(f"xc2c/gen-tile-{parts[0]['name']}-mc.html", "w") as tf:
            gen_tile(tf, "mc", device["mc_bits"])
        
        if device["has_vref"]:
            f.write(f"\n")
            f.write(f"JED mapping — MCs with IOBs\n")
            f.write(f"---------------------------\n")
            f.write(f"\n")
            f.write(f".. raw:: html\n")
            f.write(f"   :file: gen-jed-{parts[0]['name']}-mc-iob.html\n")
            with open(f"xc2c/gen-jed-{parts[0]['name']}-mc-iob.html", "w") as tf:
                gen_jed(tf, "mc", device["mc_bits"], db["jed_mc_bits_large_iob"])
            f.write(f"\n")
            f.write(f"JED mapping — MCs without IOBs\n")
            f.write(f"------------------------------\n")
            f.write(f"\n")
            f.write(f".. raw:: html\n")
            f.write(f"   :file: gen-jed-{parts[0]['name']}-mc-buried.html\n")
            with open(f"xc2c/gen-jed-{parts[0]['name']}-mc-buried.html", "w") as tf:
                gen_jed(tf, "mc", device["mc_bits"], db["jed_mc_bits_large_buried"])
        else:
            f.write(f"\n")
            f.write(f"JED mapping\n")
            f.write(f"-----------\n")
            f.write(f"\n")
            f.write(f".. raw:: html\n")
            f.write(f"   :file: gen-jed-{parts[0]['name']}-mc.html\n")
            with open(f"xc2c/gen-jed-{parts[0]['name']}-mc.html", "w") as tf:
                gen_jed(tf, "mc", device["mc_bits"], db["jed_mc_bits_small"])


        f.write(f"\n")
        f.write(f"Global bits\n")
        f.write(f"===========\n")
        f.write(f"\n")
        f.write(f".. raw:: html\n")
        f.write(f"   :file: gen-tile-{parts[0]['name']}-global.html\n")
        with open(f"xc2c/gen-tile-{parts[0]['name']}-global.html", "w") as tf:
            gen_tile(tf, "global", device["global_bits"])
        f.write(f"\n")
        f.write(f"JED mapping\n")
        f.write(f"-----------\n")
        f.write(f"\n")
        f.write(f".. raw:: html\n")
        f.write(f"   :file: gen-jed-{parts[0]['name']}-global.html\n")
        with open(f"xc2c/gen-jed-{parts[0]['name']}-global.html", "w") as tf:
            gen_jed(tf, "global", device["global_bits"], device["jed_global_bits"])
