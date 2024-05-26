import json

for kind in ["xcv", "xc2v", "xc3s", "xc6s", "xc4v", "xc5v", "xc6v", "xc7v"]:
    with open(f"../databases/{kind}-tiledb.json") as dbf:
        db = json.load(dbf)

    def emit_dev_table_bitvec(f, name):
        l = None
        for dev, data in db["device_data"].items():
            if name in data:
                l = len(data[name])
        if l is None:
            return
        f.write("<table class=\"docutils align-default\">\n")
        f.write(f"<tr><th rowspan=\"2\">Device</th><th colspan=\"{l}\">{name}</th></tr>\n")
        f.write(f"<tr>")
        for i in reversed(range(l)):
            f.write(f"<th>[{i}]</th>")
        f.write(f"</tr>\n")
        for dev, data in db["device_data"].items():
            if name in data:
                d = data[name]
                assert len(d) == l
                f.write(f"<tr><td>{dev}</td>")
                for i in reversed(range(l)):
                    f.write(f"<td>{int(d[i])}</td>")
                f.write(f"</tr>\n")
        f.write(f"</table>\n")

    for tile_name, tile in db["tiles"].items():
        num_bittiles = 0
        for item in tile.values():
            for bt, _, _ in item["bits"]:
                num_bittiles = max(bt + 1, num_bittiles)
        bt_dims = [(0, 0) for _ in range(num_bittiles)]
        for item in tile.values():
            for bt, frame, bit in item["bits"]:
                bt_dims[bt] = (
                    max(frame + 1, bt_dims[bt][0]),
                    max(bit + 1, bt_dims[bt][1]),
                )

        with open(f"xilinx/gen-xilinx-tile-{kind}-{tile_name}.html", "w") as f:
            rev = {}
            for name, item in tile.items():
                for j, bit in enumerate(item["bits"]):
                    key = tuple(bit)
                    if key not in rev:
                        rev[key] = []
                    rev[key].append((name, item, None if len(item["bits"]) == 1 else j))
            for bt, (columns, rows) in enumerate(bt_dims):
                f.write("<table class=\"docutils align-default prjcombine-tile\">\n")
                f.write(f"<tr><th rowspan=\"2\">Row</th><th colspan=\"{columns}\">Column</th></tr>")
                f.write("<tr>")
                for col in range(columns):
                    f.write(f"<th>{col}</th>")
                f.write("</tr>\n")
                for row in range(rows):
                    f.write(f"<tr><td>{row}</td>\n")
                    for col in range(columns):
                        f.write("<td>")
                        crd = (bt, col, row)
                        if crd in rev:
                            for name, item, bidx in rev[crd]:
                                title = name
                                if bidx is not None:
                                    title = f"{name}[{bidx}]"
                                if item.get("invert", False):
                                    title = f"~{title}"
                                f.write(f"<a href=\"#bits-{kind}-{tile_name}-{name}\" title=\"{title}\">{title}</a>")
                        else:
                            f.write("-")
                        f.write("</td>")
                    f.write("</tr>\n")
                f.write("</table>\n")

            for name, item in sorted(tile.items(), key=lambda x: x[1]["bits"]):
                f.write(f"<table class=\"docutils align-default prjcombine-enum\" id=\"bits-{kind}-{tile_name}-{name}\">\n")
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

    if kind == "xc3s":
        with open("xilinx/gen-xilinx-xc3s-bram-opts.html", "w") as f:
            emit_dev_table_bitvec(f, "BRAM:DDEL_A_DEFAULT")
            emit_dev_table_bitvec(f, "BRAM:DDEL_B_DEFAULT")
            emit_dev_table_bitvec(f, "BRAM:WDEL_A_DEFAULT")
            emit_dev_table_bitvec(f, "BRAM:WDEL_B_DEFAULT")
