import json

for kind in ["xc5k", "xcv", "xc2v", "xc3s", "xcexf", "xc6s", "xc4v", "xc5v", "xc6v", "xc7v"]:
    with open(f"../databases/{kind}-tiledb.json") as dbf:
        db = json.load(dbf)

    def emit_misc_table(fname, *prefixes):
        items = []
        for name, data in db["misc_data"].items():
            if name.startswith(prefixes[0] + ":"):
                name = name[len(prefixes[0])+1:]
                data = []
                lens = []
                for pref in prefixes:
                    xname = pref + ":" + name
                    d = db["misc_data"][xname]
                    if isinstance(d, list):
                        lens.append(len(d))
                    else:
                        lens.append(-1)
                    data.append(d)
                items.append((name, data))
        if not items:
            return
        with open(fname, "w") as f:
            f.write("<table class=\"docutils align-default\">\n")
            f.write(f"<tr><th rowspan=\"2\">Name</th>")
            for (pref, l) in zip(prefixes, lens):
                if l == -1:
                    f.write(f"<th rowspan=\"2\">{pref}</th>")
                else:
                    f.write(f"<th colspan=\"{l}\">{pref}</th>")
            f.write(f"</tr>\n")
            f.write(f"<tr>")
            for l in lens:
                if l != -1:
                    for i in reversed(range(l)):
                        f.write(f"<th>[{i}]</th>")
            f.write(f"</tr>\n")
            for name, data in items:
                f.write(f"<tr><td>{name}</td>")
                for (d, l) in zip(data, lens):
                    if l == -1:
                        f.write(f"<td>{d}</td>")
                    else:
                        assert len(d) == l
                        for i in reversed(range(l)):
                            f.write(f"<td>{int(d[i])}</td>")
                f.write(f"</tr>\n")
            f.write(f"</table>\n")

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

    def emit_dev_table_string(f, name):
        found = False
        for dev, data in db["device_data"].items():
            if name in data:
                found = True
                break
        if not found:
            return
        f.write("<table class=\"docutils align-default\">\n")
        f.write(f"<tr><th>Device</th><th>Value</th></tr>\n")
        for dev, data in db["device_data"].items():
            if name in data:
                f.write(f"<tr><td>{dev}</td><td>{data[name]}</td</tr>\n")
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

        with open(f"xilinx/gen/tile-{kind}-{tile_name}.html", "w") as f:
            rev = {}
            for name, item in tile.items():
                for j, bit in enumerate(item["bits"]):
                    key = tuple(bit)
                    if key not in rev:
                        rev[key] = []
                    rev[key].append((name, item, None if len(item["bits"]) == 1 else j))
            for bt, (columns, rows) in enumerate(bt_dims):
                f.write("<table class=\"docutils align-default prjcombine-tile\">\n")
                f.write(f"<tr><th colspan=\"{columns + 1}\">{tile_name} bittile {bt}</th></tr>")
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
                            first = True
                            for name, item, bidx in rev[crd]:
                                if not first:
                                    f.write(f"<br>")
                                title = name
                                if bidx is not None:
                                    title = f"{name}[{bidx}]"
                                invert = item.get("invert", False)
                                if not isinstance(invert, bool):
                                    invert = invert[bidx]
                                if invert:
                                    title = f"~{title}"
                                f.write(f"<a href=\"#bits-{kind}-{tile_name}-{name}\" title=\"{title}\">{title}</a>")
                                first = False
                        else:
                            f.write("-")
                        f.write("</td>")
                    f.write("</tr>\n")
                f.write("</table>\n")

            groups = {}
            item_to_group = {}
            for name, item in tile.items():
                bel, _, akey = name.partition(":")
                if bel.startswith("SLICE") and akey[0] in "ABCD":
                    akey = akey[1:]
                if name.startswith("SLICE") and akey.startswith("FFY"):
                    akey = "FFX" + akey[3:]
                if name.startswith("SLICE") and akey.endswith("Y"):
                    akey = akey[:-1] + "X"
                if name.startswith("SLICE") and akey.startswith("G"):
                    akey = "F" + akey[1:]
                if bel == "BRAM" and akey[-1] in "AB":
                    akey = akey[:-1]
                if bel == "BRAM" and akey == "PORTB_ATTR":
                    akey = "PORTA_ATTR"
                if bel == "BRAM" and akey.startswith("DOB"):
                    akey = "DOA" + akey[3:]
                if (bel == "MULT" or bel.startswith("DSP")) and akey.endswith("REG"):
                    akey = "REG"
                if (bel == "MULT" or bel.startswith("DSP")) and akey.endswith("MUX"):
                    akey = "MUX"
                if bel == "BRAM" and akey == "INVERT_CLK_DOB_REG":
                    akey = "INVERT_CLK_DOA_REG"
                if bel == "BRAM" and akey.startswith("READ_WIDTH"):
                    akey = "WRITE_WIDTH" + akey[10:]
                if bel == "BRAM" and akey.endswith("_OFFSET"):
                    akey = "ALMOST_OFFSET"
                if bel == "INT":
                    akey = "INT"
                if bel == "INTF" and akey.startswith("DELAY"):
                    akey = "DELAY"
                if akey.startswith("MUXBUS"):
                    akey = "MUXBUS"
                if akey.endswith("INV"):
                    akey = "INV"
                if bel == "CLK_HROW":
                    akey = "MUX"
                akey = ""
                if "values" in item:
                    vkey = str(sorted(item["values"].items(), key=lambda x: x[1][::-1]))
                else:
                    vkey = str((item["invert"], len(item["bits"])))
                key = (akey, vkey)
                item_to_group[name] = key
                groups.setdefault(key, []).append(name)

            groups_done = set()

            for name, item in sorted(tile.items(), key=lambda x: x[1]["bits"]):
                group = item_to_group[name]
                if group in groups_done:
                    continue
                groups_done.add(group)
                for iname in groups[group]:
                    f.write(f"<div id=\"bits-{kind}-{tile_name}-{iname}\"></div>\n")
                f.write(f"<table class=\"docutils align-default prjcombine-enum\">\n")
                for iname in groups[group]:
                    citem = tile[iname]
                    f.write(f"<tr><th>{iname}</th>")
                    for bit in reversed(citem["bits"]):
                        f.write(f"<th>{bit}</th>")
                    f.write("</tr>\n")
                if "values" in item:
                    for vname, val in sorted(item["values"].items(), key=lambda x: x[1][::-1]):
                        f.write(f"<tr><td>{vname}</td>")
                        for v in reversed(val):
                            f.write(f"<td>{int(v)}</td>")
                        f.write(f"</tr>\n")
                else:
                    invert = item["invert"]
                    if invert is True:
                        f.write("<tr><td>Inverted</td>")
                    elif invert is False:
                        f.write("<tr><td>Non-inverted</td>")
                    else:
                        f.write("<tr><td>Mixed inversion</td>")
                    for i in reversed(range(len(item["bits"]))):
                        if isinstance(invert, bool):
                            inv = invert
                        else:
                            inv = invert[i]
                        if inv:
                            inv = "~"
                        else:
                            inv = ""
                        f.write(f"<td>{inv}[{i}]</td>")
                    f.write("</tr>\n")
                f.write("</table>\n")

    if kind == "xcv":
        emit_misc_table("xilinx/gen/xcv-iostd-misc.html", "IOSTD:V:IOSTD_MISC", "IOSTD:V:OUTPUT_MISC")
        emit_misc_table("xilinx/gen/xcv-iostd-drive.html", "IOSTD:V:PDRIVE", "IOSTD:V:NDRIVE")
        emit_misc_table("xilinx/gen/xcv-iostd-slew.html", "IOSTD:V:SLEW")
        emit_misc_table("xilinx/gen/xcve-iostd-misc.html", "IOSTD:VE:IOSTD_MISC", "IOSTD:VE:OUTPUT_MISC")
        emit_misc_table("xilinx/gen/xcve-iostd-drive.html", "IOSTD:VE:PDRIVE", "IOSTD:VE:NDRIVE")
        emit_misc_table("xilinx/gen/xcve-iostd-slew.html", "IOSTD:VE:SLEW")

    if kind == "xc2v":
        emit_misc_table("xilinx/gen/xc2v-iostd-drive.html", "IOSTD:V2:PDRIVE", "IOSTD:V2:NDRIVE")
        emit_misc_table("xilinx/gen/xc2v-iostd-slew.html", "IOSTD:V2:SLEW")
        emit_misc_table("xilinx/gen/xc2v-iostd-output-misc.html", "IOSTD:V2:OUTPUT_MISC")
        emit_misc_table("xilinx/gen/xc2v-iostd-output-diff.html", "IOSTD:V2:OUTPUT_DIFF")
        emit_misc_table("xilinx/gen/xc2v-iostd-lvdsbias.html", "IOSTD:V2:LVDSBIAS")
        emit_misc_table("xilinx/gen/xc2v-iostd-dci-term-split.html", "IOSTD:V2:PMASK_TERM_SPLIT", "IOSTD:V2:NMASK_TERM_SPLIT")
        emit_misc_table("xilinx/gen/xc2v-iostd-dci-term-vcc.html", "IOSTD:V2:PMASK_TERM_VCC")
        emit_misc_table("xilinx/gen/xc2vp-iostd-drive.html", "IOSTD:V2P:PDRIVE", "IOSTD:V2P:NDRIVE")
        emit_misc_table("xilinx/gen/xc2vp-iostd-slew.html", "IOSTD:V2P:SLEW")
        emit_misc_table("xilinx/gen/xc2vp-iostd-output-misc.html", "IOSTD:V2P:OUTPUT_MISC")
        emit_misc_table("xilinx/gen/xc2vp-iostd-output-diff.html", "IOSTD:V2P:OUTPUT_DIFF")
        emit_misc_table("xilinx/gen/xc2vp-iostd-lvdsbias.html", "IOSTD:V2P:LVDSBIAS")
        emit_misc_table("xilinx/gen/xc2vp-iostd-dci-term-split.html", "IOSTD:V2P:PMASK_TERM_SPLIT", "IOSTD:V2P:NMASK_TERM_SPLIT")
        emit_misc_table("xilinx/gen/xc2vp-iostd-dci-term-vcc.html", "IOSTD:V2P:PMASK_TERM_VCC")
        emit_misc_table("xilinx/gen/xc2v-gt10-PMA_SPEED.html", "GT10:PMA_SPEED")

        with open("xilinx/gen/xc2v-dcm-deskew-adjust.html", "w") as f:
            emit_dev_table_bitvec(f, "DCM:DESKEW_ADJUST")

    if kind == "xc3s":
        emit_misc_table("xilinx/gen/xc3s-iostd-drive.html", "IOSTD:S3:PDRIVE", "IOSTD:S3:NDRIVE")
        emit_misc_table("xilinx/gen/xc3s-iostd-slew.html", "IOSTD:S3:SLEW")
        emit_misc_table("xilinx/gen/xc3s-iostd-output-misc.html", "IOSTD:S3:OUTPUT_MISC")
        emit_misc_table("xilinx/gen/xc3s-iostd-output-diff.html", "IOSTD:S3:OUTPUT_DIFF")
        emit_misc_table("xilinx/gen/xc3s-iostd-lvdsbias.html", "IOSTD:S3:LVDSBIAS")
        emit_misc_table("xilinx/gen/xc3s-iostd-dci-term-split.html", "IOSTD:S3:PMASK_TERM_SPLIT", "IOSTD:S3:NMASK_TERM_SPLIT")
        emit_misc_table("xilinx/gen/xc3s-iostd-dci-term-vcc.html", "IOSTD:S3:PMASK_TERM_VCC")
        emit_misc_table("xilinx/gen/xc3se-iostd-drive.html", "IOSTD:S3E:PDRIVE", "IOSTD:S3E:NDRIVE")
        emit_misc_table("xilinx/gen/xc3se-iostd-slew.html", "IOSTD:S3E:SLEW")
        emit_misc_table("xilinx/gen/xc3se-iostd-output-misc.html", "IOSTD:S3E:OUTPUT_MISC")
        emit_misc_table("xilinx/gen/xc3se-iostd-output-diff.html", "IOSTD:S3E:OUTPUT_DIFF")
        emit_misc_table("xilinx/gen/xc3se-iostd-lvdsbias.html", "IOSTD:S3E:LVDSBIAS")
        emit_misc_table("xilinx/gen/xc3sa-iostd-tb-drive.html", "IOSTD:S3A.TB:PDRIVE", "IOSTD:S3A.TB:NDRIVE")
        emit_misc_table("xilinx/gen/xc3sa-iostd-tb-slew.html", "IOSTD:S3A.TB:PSLEW", "IOSTD:S3A.TB:NSLEW")
        emit_misc_table("xilinx/gen/xc3sa-iostd-tb-output-diff.html", "IOSTD:S3A.TB:OUTPUT_DIFF")
        emit_misc_table("xilinx/gen/xc3sa-iostd-lr-drive.html", "IOSTD:S3A.LR:PDRIVE", "IOSTD:S3A.LR:NDRIVE")
        emit_misc_table("xilinx/gen/xc3sa-iostd-lr-slew.html", "IOSTD:S3A.LR:PSLEW", "IOSTD:S3A.LR:NSLEW")
        emit_misc_table("xilinx/gen/xc3sa-iostd-tb-lvdsbias.html", "IOSTD:S3A.TB:LVDSBIAS")

        with open("xilinx/gen/xc3s-bram-opts.html", "w") as f:
            emit_dev_table_bitvec(f, "BRAM:DDEL_A_DEFAULT")
            emit_dev_table_bitvec(f, "BRAM:DDEL_B_DEFAULT")
            emit_dev_table_bitvec(f, "BRAM:WDEL_A_DEFAULT")
            emit_dev_table_bitvec(f, "BRAM:WDEL_B_DEFAULT")

        with open("xilinx/gen/xc3s-pcilogicse-opts.html", "w") as f:
            emit_dev_table_string(f, "PCILOGICSE:DELAY_DEFAULT")

        with open("xilinx/gen/xc3s-dcm-deskew-adjust.html", "w") as f:
            emit_dev_table_bitvec(f, "DCM:DESKEW_ADJUST")

        intf_mux = []
        for item, val in db["misc_data"].items():
            if item.startswith("INTF.DSP:INTF_GROUP"):
                _, _, mux, inp = item.split(":")
                intf_mux.append((mux, inp, val))
        if intf_mux:
            with open("xilinx/gen/xc3s-INTF.DSP.html", "w") as f:
                f.write("<table class=\"docutils align-default\">\n")
                f.write("<tr><th>Mux</th><th>Mux input</th><th>Test group</th></tr>\n")
                for mux, inp, val in intf_mux:
                    f.write(f"<tr><td>{mux}</td><td>{inp}</td><td>{val}</td></tr>")

                f.write(f"</table>")

    if kind == "xc6s":
        with open("xilinx/gen/xc6s-pci-ce-delay.html", "w") as f:
            emit_dev_table_string(f, "PCI_CE_DELAY")
        emit_misc_table("xilinx/gen/xc6s-pll-lock.html",
                               "PLL:PLL_LOCK_REF_DLY",
                               "PLL:PLL_LOCK_FB_DLY",
                               "PLL:PLL_LOCK_CNT",
                               "PLL:PLL_LOCK_SAT_HIGH",
                               "PLL:PLL_UNLOCK_CNT")
        emit_misc_table("xilinx/gen/xc6s-pll-filter.html",
                               "PLL:PLL_CP",
                               "PLL:PLL_CP_REPL",
                               "PLL:PLL_RES",
                               "PLL:PLL_LFHF")

    if kind == "xc4v":
        emit_misc_table("xilinx/gen/xc4v-iostd-misc.html", "IOSTD:OUTPUT_MISC")
        emit_misc_table("xilinx/gen/xc4v-iostd-drive.html", "IOSTD:PDRIVE", "IOSTD:NDRIVE")
        emit_misc_table("xilinx/gen/xc4v-iostd-slew.html", "IOSTD:PSLEW", "IOSTD:NSLEW")
        emit_misc_table("xilinx/gen/xc4v-iostd-lvds.html", "IOSTD:LVDS_T", "IOSTD:LVDS_C")
        emit_misc_table("xilinx/gen/xc4v-iostd-lvdsbias.html", "IOSTD:LVDSBIAS")
        emit_misc_table("xilinx/gen/xc4v-iostd-dci-lvdiv2.html", "IOSTD:DCI:LVDIV2")
        emit_misc_table("xilinx/gen/xc4v-iostd-dci-mask-term-vcc.html", "IOSTD:DCI:PMASK_TERM_VCC")
        emit_misc_table("xilinx/gen/xc4v-iostd-dci-mask-term-split.html", "IOSTD:DCI:PMASK_TERM_SPLIT", "IOSTD:DCI:NMASK_TERM_SPLIT")

    if kind == "xc5v":
        emit_misc_table("xilinx/gen/xc5v-iostd-misc.html", "IOSTD:OUTPUT_MISC")
        emit_misc_table("xilinx/gen/xc5v-iostd-drive.html", "IOSTD:PDRIVE", "IOSTD:NDRIVE")
        emit_misc_table("xilinx/gen/xc5v-iostd-slew.html", "IOSTD:PSLEW", "IOSTD:NSLEW")
        emit_misc_table("xilinx/gen/xc5v-iostd-lvds.html", "IOSTD:LVDS_T", "IOSTD:LVDS_C")
        emit_misc_table("xilinx/gen/xc5v-iostd-lvdsbias.html", "IOSTD:LVDSBIAS")
        emit_misc_table("xilinx/gen/xc5v-iostd-dci-lvdiv2.html", "IOSTD:DCI:LVDIV2")
        emit_misc_table("xilinx/gen/xc5v-iostd-dci-mask-term-vcc.html", "IOSTD:DCI:PMASK_TERM_VCC")
        emit_misc_table("xilinx/gen/xc5v-iostd-dci-mask-term-split.html", "IOSTD:DCI:PMASK_TERM_SPLIT", "IOSTD:DCI:NMASK_TERM_SPLIT")

        with open("xilinx/gen/xc5v-iodelay-default-idelay-value.html", "w") as f:
            emit_dev_table_bitvec(f, "IODELAY:DEFAULT_IDELAY_VALUE")

        emit_misc_table("xilinx/gen/xc5v-pll-filter.html",
                               "PLL:PLL_CP",
                               "PLL:PLL_RES",
                               "PLL:PLL_LFHF")
        with open("xilinx/gen/xc5v-pll-in-dly-set.html", "w") as f:
            emit_dev_table_bitvec(f, "PLL:PLL_IN_DLY_SET")

    if kind == "xc6v":
        emit_misc_table("xilinx/gen/xc6v-iostd-misc.html", "IOSTD:OUTPUT_MISC")
        emit_misc_table("xilinx/gen/xc6v-iostd-drive.html", "IOSTD:PDRIVE", "IOSTD:NDRIVE")
        emit_misc_table("xilinx/gen/xc6v-iostd-slew.html", "IOSTD:PSLEW", "IOSTD:NSLEW")
        emit_misc_table("xilinx/gen/xc6v-iostd-lvds.html", "IOSTD:LVDS_T", "IOSTD:LVDS_C")
        emit_misc_table("xilinx/gen/xc6v-iostd-lvdsbias.html", "IOSTD:LVDSBIAS")
        emit_misc_table("xilinx/gen/xc6v-iostd-dci-output.html", "IOSTD:DCI:PREF_OUTPUT", "IOSTD:DCI:NREF_OUTPUT")
        emit_misc_table("xilinx/gen/xc6v-iostd-dci-output-half.html", "IOSTD:DCI:PREF_OUTPUT_HALF", "IOSTD:DCI:NREF_OUTPUT_HALF")
        emit_misc_table("xilinx/gen/xc6v-iostd-dci-term-vcc.html", "IOSTD:DCI:PREF_TERM_VCC", "IOSTD:DCI:PMASK_TERM_VCC")
        emit_misc_table("xilinx/gen/xc6v-iostd-dci-term-split.html", "IOSTD:DCI:PREF_TERM_SPLIT", "IOSTD:DCI:NREF_TERM_SPLIT", "IOSTD:DCI:PMASK_TERM_SPLIT", "IOSTD:DCI:NMASK_TERM_SPLIT")

        with open("xilinx/gen/xc6v-iodelay-default-idelay-value.html", "w") as f:
            emit_dev_table_bitvec(f, "IODELAY:DEFAULT_IDELAY_VALUE")

        emit_misc_table("xilinx/gen/xc6v-mmcm-filter.html", "MMCM:CP", "MMCM:RES", "MMCM:LFHF")
        emit_misc_table("xilinx/gen/xc6v-mmcm-lock.html",
                        "MMCM:LOCK_REF_DLY",
                        "MMCM:LOCK_FB_DLY",
                        "MMCM:LOCK_CNT",
                        "MMCM:LOCK_SAT_HIGH",
                        "MMCM:UNLOCK_CNT")
        with open("xilinx/gen/xc6v-mmcm-in-dly-set.html", "w") as f:
            emit_dev_table_bitvec(f, "MMCM:IN_DLY_SET")

    if kind == "xc7v":
        # TODO: IO

        emit_misc_table("xilinx/gen/xc7v-mmcm-filter.html", "MMCM:CP", "MMCM:RES", "MMCM:LFHF")
        emit_misc_table("xilinx/gen/xc7v-mmcm-lock.html",
                        "MMCM:LOCK_REF_DLY",
                        "MMCM:LOCK_FB_DLY",
                        "MMCM:LOCK_CNT",
                        "MMCM:LOCK_SAT_HIGH",
                        "MMCM:UNLOCK_CNT")

        emit_misc_table("xilinx/gen/xc7v-pll-filter.html", "PLL:CP", "PLL:RES", "PLL:LFHF")
        emit_misc_table("xilinx/gen/xc7v-pll-lock.html",
                        "PLL:LOCK_REF_DLY",
                        "PLL:LOCK_FB_DLY",
                        "PLL:LOCK_CNT",
                        "PLL:LOCK_SAT_HIGH",
                        "PLL:UNLOCK_CNT")
