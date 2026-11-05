#!/usr/bin/env python
import argparse
import hashlib
import os
import requests
import re
import zipfile


def download_file(url, file_path):
    with open(file_path, "ab") as f:
        resume_byte_pos = f.tell()
        resume_header = {"Range": "bytes={}-".format(resume_byte_pos)}
        resp = requests.get(
            url, headers=resume_header, stream=True,
            verify=False, allow_redirects=True
        )

        if resp.status_code == 200 or resp.status_code == 206:
            for chunk in resp.iter_content(4096000):
                f.write(chunk)


def extract_file_from_zip(path, zip_file, output_file):
    with zipfile.ZipFile(zip_file) as z:
        with z.open(path) as zf, open(output_file, "wb") as f:
            f.write(zf.read())


def verify_file(path, checksum):
    with open(path, "rb") as f:
        sha1 = hashlib.sha1()
        sha1.update(f.read())

    return sha1.hexdigest() == checksum


def download_with_check(url, checksum, file):
    if not verify_file(file, checksum):
        download_file(url, file)


def icrp107():
    data_url = "https://journals.sagepub.com/doi/suppl/10.1177/ANIB_38_3/suppl_file/P107JAICRP_38_3_Nuclear_Decay_Data_suppl_data.zip"
    data_checksum = "7c9dacf10da430228e66777c954885abf4267c71"
    data_file_name = "P107JAICRP_38_3_Nuclear_Decay_Data_suppl_data.zip"
    download_with_check(data_url, data_checksum, data_file_name)

    base_path = "P 107 JAICRP 38(3) Nuclear Decay Data for Dosimetric Calculations(supplementary data)/"
    base_output_path = "icrp107/"
    for file in [
        "ICRP-07.NDX", "ICRP-07.RAD", "ICRP-07.BET", "ICRP-07.NSF", "ICRP-07.ACK"
    ]:
        extract_file_from_zip(
            base_path + file, data_file_name, base_output_path + file
        )

    corr_url = "https://www.icrp.org/docs/Corrigenda%20of%20Publication%20107.zip"
    corr_checksum = "beede0b46b73b0f1d521383620167368ca0d3a04"
    corr_file_name = "Corrigenda of Publication 107.zip"
    download_with_check(corr_url, corr_checksum, corr_file_name)

    extract_file_from_zip(
        "Corrigenda of Publication 107/ICRP-07.NDX",
        corr_file_name,
        base_output_path + "ICRP-07.NDX"
    )


def nist_material_constants():
    materials_url = "https://physics.nist.gov/PhysRefData/XrayMassCoef/tab1.html"
    html = re.sub(r"\r?\n", "", requests.get(materials_url).text)
    html_table = re.search(r"<TABLE.*>.*</TABLE>", html).group()
    html_rows = re.findall(r"<TR>(.*?)</TR>", html_table)[3:]

    table = [
        ["Z", "Symbol", "Element", "Z/A", "I", "Density"],
        ["", "", "", "", "(eV)", "(g/cm3)"],
    ]
    for row in html_rows:
        table.append([cell.strip() for cell in re.findall(
            r"<TD.*?>(.*?)</TD>", row) if cell != "&nbsp;"])

    return "\n".join(["{:4}{:8}{:18}{:10}{:10}{:10}".format(*r) for r in table])


def nist_elemental_media(z):
    base_url = "https://physics.nist.gov/PhysRefData/XrayMassCoef/ElemTab/z{:02}.html"
    html = requests.get(base_url.format(z)).text
    html_pre = re.search(
        r"(?:<PRE>)((\r|\n|.)*)(?:</PRE>)", html).group()

    table = [
        ["Energy", "mu/rho", "mu_en/rho"],
        ["(MeV)", "(cm2/g)", "(cm2/g)"],
    ]
    for row in re.finditer(r"(\d\.\d+?E[+-]\d{2}\s+){3}", html_pre):
        table.append(row.group().split())

    return "\n".join(["{:12}{:12}{:12}".format(*r) for r in table])


def nist_mass_attenuation_coefficient():
    data_path = "XrayMassAttenCoef"
    os.makedirs(data_path, exist_ok=True)

    with open("{}/material_constants".format(data_path), "w") as f:
        f.write(nist_material_constants())

    for z in range(1, 93):
        with open("{}/{:02}".format(data_path, z), "w") as f:
            f.write(nist_elemental_media(z))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Nuclide datasets download script"
    )

    all_datasets = ["icrp107", "nist_mass_attenuation_coefficient"]

    parser.add_argument(
        "--dataset",
        nargs="*",
        choices=["all"] + all_datasets,
        default="all",
        help="specify datasets to download (default: all)"
    )

    args = parser.parse_args()

    datasets = all_datasets if "all" in args.dataset else args.dataset
    for ds in datasets:
        if ds == "icrp107":
            icrp107()
        elif ds == "nist_mass_attenuation_coefficient":
            nist_mass_attenuation_coefficient()
