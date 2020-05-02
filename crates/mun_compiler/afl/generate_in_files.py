#!/usr/bin/python3
from pathlib import Path
import re


def find_snaps(folder: str):
    """Find all snap files in directories"""
    return Path(folder).rglob("*.snap")


def extract_snippet(data: str):
    """Extract snippets in the snap file
       Will be in the form of:
        expression: \" <expression> \"
    """
    result = re.findall('(expression: "(pub|fn|struct).*)', data)
    if len(result) > 0:
        data = result[0][0]
        # Remove the expression part and replace newlines, also trim the final "
        return data.replace('expression: "', "").replace("\\n", "\n")[0:-1]


def generate_files(folder: str, out_folder: str):
    """Generate files according to snap files"""
    snippets = []
    for path in find_snaps(folder):
        with open(path, "r") as f:
            data = f.read()
            snippet = extract_snippet(data)
            if snippet:
                snippets.append(extract_snippet(data))
    for (index, snippet) in enumerate(snippets):
        print(snippet)
        with open("{}/{}".format(out_folder, index), "w") as f:
            print(snippet)
            f.write(snippet)



if __name__ == "__main__":
    generate_files("../../", "in")
