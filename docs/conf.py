# SPDX-License-Identifier: copyleft-next-0.3.1
# Configuration file for the Sphinx documentation builder.
import os
import re
import subprocess

from docutils import nodes

project = "qemu-system-generator"
copyright = "2026, qemu-system-generator authors"
author = "qemu-system-generator authors"

extensions = [
    "sphinx_copybutton",
]

# Show the "$ " host / "# " guest prompt but strip it (and follow "\"
# continuations) on copy.
copybutton_prompt_text = r"[$#] "
copybutton_prompt_is_regexp = True
copybutton_line_continuation_character = "\\"

# A guide includes the bootstrap/_*.rst fragments; they are no page of their own.
exclude_patterns = [
    "_build",
    "_generated",
    "bootstrap/_*.rst",
]

html_theme = "pydata_sphinx_theme"
html_baseurl = os.environ.get("READTHEDOCS_CANONICAL_URL", "/")
html_static_path = ["_static"]
html_css_files = ["custom.css"]
html_favicon = "_static/favicon.ico"
html_theme_options = {
    "logo": {
        "image_light": "_static/logo.png",
        "image_dark": "_static/logo.png",
        "alt_text": "qemu-system-generator",
    },
    "navigation_with_keys": False,
    "navbar_align": "left",
    "header_links_before_dropdown": 8,
    "icon_links": [
        {
            "name": "GitHub",
            "url": "https://github.com/linux-kdevops/qemu-system-generator",
            "icon": "fa-brands fa-github",
        },
        {
            "name": "Discord",
            "url": "https://bit.ly/linux-kdevops-chat",
            "icon": "fa-brands fa-discord",
        },
        {
            "name": "IRC: #kdevops on OFTC",
            "url": "https://webchat.oftc.net/?channels=kdevops",
            "icon": "fa-solid fa-comments",
        },
        {
            "name": "Mailing list: kdevops@lists.linux.dev",
            "url": "mailto:kdevops@lists.linux.dev",
            "icon": "fa-solid fa-envelope",
        },
    ],
}
html_context = {
    "default_mode": "light",
}

# Monospaced external links. ":cmd:`name`" renders ``name`` (a literal, so it
# reads as a command) hyperlinked to its manual or source. The table is the one
# source for every command, tool and manual page URL the docs cite, so a name
# is linked the same way everywhere and updated in one place.
_SYSTEMD = "https://www.freedesktop.org/software/systemd/man/latest"
_MAN1 = "https://man7.org/linux/man-pages/man1"
_NIX = "https://nix.dev/manual/nix/latest/command-ref/new-cli"
cmd_links = {
    "systemctl": f"{_SYSTEMD}/systemctl.html",
    "journalctl": f"{_SYSTEMD}/journalctl.html",
    "machinectl": f"{_SYSTEMD}/machinectl.html",
    "systemd.generator(7)": f"{_SYSTEMD}/systemd.generator.html",
    "systemd.path(5)": f"{_SYSTEMD}/systemd.path.html",
    "systemd-machined.service(8)": f"{_SYSTEMD}/systemd-machined.service.html",
    "systemd-ssh-generator(8)": f"{_SYSTEMD}/systemd-ssh-generator.html",
    "systemd-ssh-proxy(1)": f"{_SYSTEMD}/systemd-ssh-proxy.html",
    "machines.target": f"{_SYSTEMD}/systemd.special.html",
    "ssh": f"{_MAN1}/ssh.1.html",
    "ssh-keygen": f"{_MAN1}/ssh-keygen.1.html",
    "systemd.exec(5)": f"{_SYSTEMD}/systemd.exec.html",
    "readlink": f"{_MAN1}/readlink.1.html",
    "socat": "http://www.dest-unreach.org/socat/doc/socat.html",
    "jq": "https://jqlang.org/manual/",
    "curl": "https://curl.se/docs/manpage.html",
    "cloud-localds": "https://manpages.debian.org/unstable/cloud-image-utils/cloud-localds.1.en.html",
    "qemu-system-x86_64(1)": "https://www.qemu.org/docs/master/system/invocation.html",
    "qemu-guest.nix": "https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/profiles/qemu-guest.nix",
    "nix build": f"{_NIX}/nix3-build",
    "nix develop": f"{_NIX}/nix3-develop",
    "nix flake init": f"{_NIX}/nix3-flake-init",
    "nix profile add": f"{_NIX}/nix3-profile-add",
}


def _cmd_role(name, rawtext, text, lineno, inliner, options=None, content=None):
    uri = inliner.document.settings.env.config.cmd_links.get(text)
    code = nodes.literal(rawtext, text)
    if uri is None:
        msg = inliner.reporter.error(
            f"cmd: no link registered for {text!r} (add it to cmd_links in "
            f"docs/conf.py)",
            line=lineno,
        )
        return [code], [msg]
    return [nodes.reference(rawtext, "", code, refuri=uri)], []


# ":commit:`<hash>`" links a Linux commit and ":repo:`<path>`" a file of this
# repository, each rendered as a literal like ":cmd:".
_LINKS = {
    "commit": "https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/commit/?id={}",
    "repo": "https://github.com/linux-kdevops/qemu-system-generator/blob/main/{}",
}


def _link_role(name, rawtext, text, lineno, inliner, options=None, content=None):
    code = nodes.literal(rawtext, text)
    return [nodes.reference(rawtext, "", code, refuri=_LINKS[name].format(text))], []


# The man pages stay in scdoc under man/, where the package builds them. Each
# docs/man/ page includes its man page as HTML that scdoc and mandoc render
# here, so the web page and the installed page come from one source. mandoc
# heads every section with <h1>, which would compete with the page title.
_DOCS = os.path.dirname(os.path.abspath(__file__))
_MAN = os.path.join(os.path.dirname(_DOCS), "man")
_GENERATED = os.path.join(_DOCS, "_generated")


def _render_man_pages(app):
    os.makedirs(_GENERATED, exist_ok=True)
    for page in sorted(os.listdir(_MAN)):
        if not page.endswith(".scd"):
            continue
        with open(os.path.join(_MAN, page), "rb") as scd:
            roff = subprocess.run(
                ["scdoc"], stdin=scd, capture_output=True, check=True
            ).stdout
        html = subprocess.run(
            ["mandoc", "-T", "html", "-O", "fragment"],
            input=roff,
            capture_output=True,
            check=True,
        ).stdout.decode()
        html = re.sub(r"<(/?)h1\b", r"<\1h2", html)
        with open(os.path.join(_GENERATED, page[: -len(".scd")] + ".html"), "w") as out:
            out.write(html)


def setup(app):
    app.add_config_value("cmd_links", {}, "env")
    app.add_role("cmd", _cmd_role)
    for name in _LINKS:
        app.add_role(name, _link_role)
    app.connect("builder-inited", _render_man_pages)
    return {"parallel_read_safe": True, "parallel_write_safe": True}
