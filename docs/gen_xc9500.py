import json

with open("../databases/xc9500.json") as f:
    db_p = json.load(f)
with open("../databases/xc9500xl.json") as f:
    db_xl = json.load(f)
with open("../databases/xc9500xv.json") as f:
    db_xv = json.load(f)

dbs = [db_p, db_xl, db_xv]

with open("xc9500/gen-devices.inc", "w") as f:
    f.write(".. list-table::\n")
    f.write("   :header-rows: 1\n")
    f.write("\n")
    f.write("   - - Device\n")
    f.write("     - Variant\n")
    f.write("     - IDCODE\n")
    f.write("     - Function Blocks\n")
    f.write("     - GOE pins / FOE networks\n")
    f.write("     - I/O banks\n")
    f.write("     - Notes\n")
    for db in dbs:
        for part in db["parts"]:
            device = db["devices"][part["device"]]
            notes = ""
            if device["kind"] == "xc9500" and device["fbs"] == 2:
                notes = "Does not have FB input feedback"
            if device["uim_ibuf_bits"] is not None:
                notes = "Has special input buffer enable fuses"
            if device["fbs"] == 4:
                notes = "GOE mapping to pads varies with package"
            goe_num = 0
            for key in device["io_special"]:
                if key.startswith("GOE"):
                    goe_num += 1
            f.write(f"   - - {part['name']}\n")
            f.write(f"     - {device['kind']}\n")
            f.write(f"     - ``{device['idcode']:#010x}``\n")
            f.write(f"     - {device['fbs']}\n")
            f.write(f"     - {goe_num}\n")
            f.write(f"     - {device['banks']}\n")
            f.write(f"     - {notes}\n")

packages = []
for db in dbs:
    for part in db["parts"]:
        for pkg in part["packages"]:
            if pkg not in packages:
                packages.append(pkg)

with open("xc9500/gen-devices-pkg.inc", "w") as f:
    f.write(".. list-table::\n")
    f.write("   :header-rows: 1\n")
    f.write("\n")
    f.write("   - - Device\n")
    for pkg in packages:
        f.write(f"     - {pkg}\n")
    for db in dbs:
        for part in db["parts"]:
            f.write(f"   - - {part['name']}\n")
            for pkg in packages:
                if pkg in part["packages"]:
                    f.write(f"     - X\n")
                else:
                    f.write(f"     - \\-\n")

def gen_tile_items(f, tname, tile, sortflip):
    for name, item in sorted(tile.items(), key=lambda x: x[1]["bits"]):
        f.write(f"<table class=\"docutils align-default prjcombine-enum\" id=\"bits-{tname}-{name}\">\n")
        f.write(f"<tr><th>{name}</th>")
        for bit in reversed(item["bits"]):
            f.write(f"<th>{bit}</th>")
        f.write("</tr>\n")
        if "values" in item:
            for vname, val in sorted(item["values"].items(), key=lambda x: [y ^ sortflip for y in x[1][::-1]]):
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


def gen_tile_mc(f, tname, tile, sortflip):
    f.write("<table class=\"docutils align-default prjcombine-tile\">\n")
    f.write("<tr><th>Row</th><th>Bit</th></tr>\n")
    minrow = min(bit for item in tile.values() for bit in item["bits"])
    maxrow = max(bit for item in tile.values() for bit in item["bits"])
    rev = {
        bit: (name, item, None if len(item["bits"]) == 1 else j)
        for name, item in tile.items() for j, bit in enumerate(item["bits"])
    }
    for i in range(minrow, maxrow + 1):
        f.write(f"<tr><td>{i}</td><td>")
        if i in rev:
            name, item, bit = rev[i]
            f.write(f"<a href=\"#bits-{tname}-{name}\">")
            if item.get("invert", False):
                f.write("~")
            f.write(f"{name}")
            if bit is not None:
                f.write(f"[{bit}]")
            f.write(f"</a>")
        else:
            f.write("-")
        f.write("</td></tr>\n")
    f.write("</table>\n")

    gen_tile_items(f, tname, tile, sortflip)

def gen_tile_fb(f, tname, tile, sortflip, multi=False):
    f.write("<table class=\"docutils align-default prjcombine-tile\">\n")
    if multi:
        rows = {tuple(bit[0:2]) for item in tile.values() for bit in item["bits"]}
    else:
        rows = {bit[0] for item in tile.values() for bit in item["bits"]}
    rev = {}
    for name, item in tile.items():
        for j, bit in enumerate(item["bits"]):
            key = tuple(bit)
            if key not in rev:
                rev[key] = []
            rev[key].append((name, item, None if len(item["bits"]) == 1 else j))
    if multi:
        f.write("<tr><th rowspan=\"3\">FB</th><th rowspan=\"3\">Row</th><th colspan=\"18\">Bit, column</th></tr>\n")
    else:
        f.write("<tr><th rowspan=\"3\">Row</th><th colspan=\"18\">Bit, column</th></tr>\n")
    f.write("<tr><th colspan=\"9\">6</th><th colspan=\"9\">7</th></tr>\n")
    f.write("<tr>")
    for _ in [6, 7]:
        for col in range(9):
            f.write(f"<th>{col}</th>")
    f.write("</tr>\n")
    for row in sorted(rows):
        if multi:
            fb, row = row
            f.write(f"<tr><td>{fb}</td><td>{row}</td>\n")
        else:
            f.write(f"<tr><td>{row}</td>\n")
        for bit in [6, 7]:
            for col in range(9):
                f.write("<td>")
                if multi:
                    crd = (fb, row, bit, col)
                else:
                    crd = (row, bit, col)
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

    gen_tile_items(f, tname, tile, sortflip)


with open("xc9500/gen-tile-xc9500-mc.html", "w") as f:
    gen_tile_mc(f, "mc", db_p["mc_bits"], True)

with open("xc9500/gen-tile-xc9500-fb.html", "w") as f:
    gen_tile_fb(f, "fb", db_p["fb_bits"], True)

with open("xc9500/gen-tile-xc9500-global.html", "w") as f:
    gen_tile_fb(f, "global", db_p["global_bits"], True, True)

with open("xc9500/gen-tile-xc9500xl-mc.html", "w") as f:
    gen_tile_mc(f, "mc", db_xl["mc_bits"], False)

with open("xc9500/gen-tile-xc9500xl-fb.html", "w") as f:
    gen_tile_fb(f, "fb", db_xl["fb_bits"], False)

with open("xc9500/gen-tile-xc9500xl-global.html", "w") as f:
    gen_tile_fb(f, "global", db_xv["global_bits"], False, True)

with open("xc9500/db-devices.rst", "w") as f:
    f.write("Database — devices\n")
    f.write("##################\n")
    f.write("\n")
    f.write(".. toctree::\n")
    f.write("   :caption: Contents:\n")
    f.write("\n")
    for db in dbs:
        devs_done = set()
        for part in db["parts"]:
            if part["device"] not in devs_done:
                devs_done.add(part["device"])
                f.write(f"   db-device-{part['name']}\n")

for db in dbs:
    for i, device in enumerate(db["devices"]):
        parts = []
        bonds = {}
        for part in db["parts"]:
            if part["device"] == i:
                parts.append(part)
                for pkg, bond in part["packages"].items():
                    if bond not in bonds:
                        pins = {}
                        io_special = {**device["io_special"], **db["bonds"][bond]["io_special_override"]}
                        io_special_rev = {f"IOB_{v[0]}_{v[1]}": k for k, v in io_special.items()}
                        for k, v in db["bonds"][bond]["pins"].items():
                            if v not in pins:
                                pins[v] = ([], io_special_rev.get(v))
                            pins[v][0].append(k)
                        bonds[bond] = ([], pins)
                    bonds[bond][0].append(f"{part['name']}-{pkg}")
        bonds = {
            k: bonds[k]
            for k in sorted(bonds)
        }
        dev_packages = [pkg for pkg in packages if any(pkg in part["packages"] for part in parts)]
        with open(f"xc9500/db-device-{parts[0]['name']}.rst", "w") as f:
            names = ", ".join(part["name"].upper() for part in parts)
            f.write(f"{names}\n")
            l = "#" * len(names)
            f.write(f"{l}\n")
            f.write(f"\n")
            f.write(f"IDCODE: ``{device['idcode']:#010x}``\n")
            f.write(f"\n")
            f.write(f"FB count: {device['fbs']}\n")
            f.write(f"\n")
            f.write(f"I/O bank count: {device['banks']}\n")
            f.write(f"\n")
            f.write(f"FPGM/FPGMI time: {device['program_time']}µs\n")
            f.write(f"\n")
            f.write(f"FERASE/FBULK time: {device['erase_time']}µs\n")
            f.write(f"\n")
            f.write(f"I/O pins\n")
            f.write(f"========\n")
            f.write(f"\n")
            f.write(f".. list-table::\n")
            f.write(f"   :header-rows: 1\n")
            f.write(f"\n")
            f.write(f"   - - Function\n")
            f.write(f"     - Bank\n")
            for names, _ in bonds.values():
                f.write(f"     - {', '.join(names)}\n")
            for io, bank in sorted(device["ios"].items(), key=lambda io: (int(io[0].split("_")[1]), int(io[0].split("_")[2]))):
                f.write(f"   - - {io}\n")
                f.write(f"     - {bank}\n")
                for _, pins in bonds.values():
                    if io in pins:
                        spec = pins[io][1]
                        if spec is not None:
                            f.write(f"     - {pins[io][0][0]} ({spec})\n")
                        else:
                            f.write(f"     - {pins[io][0][0]}\n")
                    else:
                        f.write(f"     - \\-\n")
            for pad in ["TCK", "TMS", "TDI", "TDO"]:
                f.write(f"   - - {pad}\n")
                if pad == "TDO":
                    f.write(f"     - {device['tdo_bank']}\n")
                else:
                    f.write(f"     - \\-\n")
                for _, pins in bonds.values():
                    f.write(f"     - {pins[pad][0][0]}\n")
            specs = ["GND", "VCCINT"]
            for bank in range(device["banks"]):
                specs.append(f"VCCIO{bank}")
            specs.append("NC")
            for spec in specs:
                f.write(f"   - - {spec}\n")
                if spec.startswith("VCCIO"):
                    f.write(f"     - {spec[5:]}\n")
                else:
                    f.write(f"     - \\-\n")
                for _, pins in bonds.values():
                    if spec in pins:
                        pins = pins[spec][0]
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
            with open(f"xc9500/gen-tile-{parts[0]['name']}-imux.html", "w") as tf:
                gen_tile_fb(tf, "imux", device["imux_bits"], device["kind"] == "xc9500")
            if device["uim_ibuf_bits"] is not None:
                f.write(f"\n")
                f.write(f"UIM IBUF bits\n")
                f.write(f"=============\n")
                f.write(f"\n")
                f.write(f".. raw:: html\n")
                f.write(f"   :file: gen-tile-{parts[0]['name']}-uim-ibuf.html\n")
                with open(f"xc9500/gen-tile-{parts[0]['name']}-uim-ibuf.html", "w") as tf:
                    gen_tile_fb(tf, "imux", device["uim_ibuf_bits"], device["kind"] == "xc9500", True)
