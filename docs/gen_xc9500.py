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
                    f.write(f"     - \-\n")

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
                        io_special_rev = {f"MC_{v[0]}_{v[1]}": k for k, v in io_special.items()}
                        # TODO io_special
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
            for io, bank in sorted(device["ios"].items(), key=lambda io: (int(io[0].split(".")[0]), int(io[0].split(".")[1]))):
                fb, mc = io.split(".")
                pad = f"MC_{fb}_{mc}"
                f.write(f"   - - {pad}\n")
                f.write(f"     - {bank}\n")
                for _, pins in bonds.values():
                    if pad in pins:
                        spec = pins[pad][1]
                        if spec is not None:
                            f.write(f"     - {pins[pad][0][0]} ({spec})\n")
                        else:
                            f.write(f"     - {pins[pad][0][0]}\n")
                    else:
                        f.write(f"     - \-\n")
            for pad in ["TCK", "TMS", "TDI", "TDO"]:
                f.write(f"   - - {pad}\n")
                if pad == "TDO":
                    f.write(f"     - {device['tdo_bank']}\n")
                else:
                    f.write(f"     - \-\n")
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
                    f.write(f"     - \-\n")
                for _, pins in bonds.values():
                    if spec in pins:
                        pins = pins[spec][0]
                        f.write(f"     - {pins[0]}\n")
                        for pin in pins[1:]:
                            f.write(f"\n")
                            f.write(f"       {pin}\n")
                    else:
                        f.write(f"     - \-\n")
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
