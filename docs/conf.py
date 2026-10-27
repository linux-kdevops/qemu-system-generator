# SPDX-License-Identifier: copyleft-next-0.3.1
# Configuration file for the Sphinx documentation builder.
import os
import re
import subprocess

from docutils import nodes
from sphinx.util import logging

logger = logging.getLogger(__name__)

project = "qemu-system-generator"
copyright = "2026, qemu-system-generator authors"
author = "qemu-system-generator authors"

extensions = [
    "sphinx_copybutton",
    "sphinx_design",
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
# reads as a command) hyperlinked to its manual or source, and ":cmd:`title
# <name>`" renders ``title`` linked as ``name``, so ``KillMode=mixed`` links
# to ``KillMode=``. The table is the one source for every command, tool,
# manual page, systemd unit and directive URL the docs cite, so a name is
# linked the same way everywhere and updated in one place.
_SYSTEMD = "https://www.freedesktop.org/software/systemd/man/latest"
_MAN1 = "https://man7.org/linux/man-pages/man1"
_NIX = "https://nix.dev/manual/nix/latest/command-ref/new-cli"
cmd_links = {
    "systemctl": f"{_SYSTEMD}/systemctl.html",
    "journalctl": f"{_SYSTEMD}/journalctl.html",
    "machinectl": f"{_SYSTEMD}/machinectl.html",
    "loginctl": f"{_SYSTEMD}/loginctl.html",
    "varlinkctl": f"{_SYSTEMD}/varlinkctl.html",
    "journald.conf(5)": f"{_SYSTEMD}/journald.conf.html",
    "systemd-journal-remote.service(8)": f"{_SYSTEMD}/systemd-journal-remote.service.html",
    "systemd-journal-upload.service(8)": f"{_SYSTEMD}/systemd-journal-upload.service.html",
    "systemd-journal-gatewayd.service(8)": f"{_SYSTEMD}/systemd-journal-gatewayd.service.html",
    "systemd.system-credentials(7)": f"{_SYSTEMD}/systemd.system-credentials.html",
    "smbios-type-11(7)": f"{_SYSTEMD}/smbios-type-11.html",
    "systemd.generator(7)": f"{_SYSTEMD}/systemd.generator.html",
    "systemd.path(5)": f"{_SYSTEMD}/systemd.path.html",
    "systemd-machined.service(8)": f"{_SYSTEMD}/systemd-machined.service.html",
    "systemd-ssh-generator(8)": f"{_SYSTEMD}/systemd-ssh-generator.html",
    "kernel-command-line(7)": f"{_SYSTEMD}/kernel-command-line.html",
    "systemd(1)": f"{_SYSTEMD}/systemd.html",
    "systemd-analyze(1)": f"{_SYSTEMD}/systemd-analyze.html",
    "systemd-ssh-proxy(1)": f"{_SYSTEMD}/systemd-ssh-proxy.html",
    "machines.target": f"{_SYSTEMD}/systemd.special.html#machines.target",
    "machine.slice": f"{_SYSTEMD}/systemd.special.html#machine.slice",
    "systemd-machined": f"{_SYSTEMD}/systemd-machined.service.html",
    "systemd-journald.service(8)": f"{_SYSTEMD}/systemd-journald.service.html",
    "systemd-logind.service(8)": f"{_SYSTEMD}/systemd-logind.service.html",
    "sshd(8)": "https://man7.org/linux/man-pages/man8/sshd.8.html",
    "systemd-path": f"{_SYSTEMD}/systemd-path.html",
    "systemd.service(5)": f"{_SYSTEMD}/systemd.service.html",
    "systemd.socket(5)": f"{_SYSTEMD}/systemd.socket.html",
    "systemd.unit(5)": f"{_SYSTEMD}/systemd.unit.html",
    "sd_notify(3)": f"{_SYSTEMD}/sd_notify.html",
    "podman-systemd.unit(5)": "https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html",
    "vsock(7)": "https://man7.org/linux/man-pages/man7/vsock.7.html",
    "Type=": f"{_SYSTEMD}/systemd.service.html#Type=",
    "ExecStart=": f"{_SYSTEMD}/systemd.service.html#ExecStart=",
    "ExecStartPost=": f"{_SYSTEMD}/systemd.service.html#ExecStartPre=",
    "ExecStop=": f"{_SYSTEMD}/systemd.service.html#ExecStop=",
    "TimeoutStopSec=": f"{_SYSTEMD}/systemd.service.html#TimeoutStopSec=",
    "KillSignal=": f"{_SYSTEMD}/systemd.kill.html#KillSignal=",
    "KillMode=": f"{_SYSTEMD}/systemd.kill.html#KillMode=",
    "PartOf=": f"{_SYSTEMD}/systemd.unit.html#PartOf=",
    "Before=": f"{_SYSTEMD}/systemd.unit.html#Before=",
    "WantedBy=": f"{_SYSTEMD}/systemd.unit.html#WantedBy=",
    "Slice=": f"{_SYSTEMD}/systemd.resource-control.html#Slice=",
    "DevicePolicy=": f"{_SYSTEMD}/systemd.resource-control.html#DevicePolicy=auto%7Cclosed%7Cstrict",
    "DeviceAllow=": f"{_SYSTEMD}/systemd.resource-control.html#DeviceAllow=",
    "MemoryMax=": f"{_SYSTEMD}/systemd.resource-control.html#MemoryMax=bytes",
    "CPUWeight=": f"{_SYSTEMD}/systemd.resource-control.html#CPUWeight=weight",
    "LimitMEMLOCK=": f"{_SYSTEMD}/systemd.exec.html#LimitCPU=",
    "StateDirectory=": f"{_SYSTEMD}/systemd.exec.html#RuntimeDirectory=",
    "RuntimeDirectory=": f"{_SYSTEMD}/systemd.exec.html#RuntimeDirectory=",
    "WorkingDirectory=": f"{_SYSTEMD}/systemd.exec.html#WorkingDirectory=",
    "ssh": f"{_MAN1}/ssh.1.html",
    "ssh-keygen": f"{_MAN1}/ssh-keygen.1.html",
    "ssh_config(5)": "https://man7.org/linux/man-pages/man5/ssh_config.5.html",
    "sshd_config(5)": "https://man7.org/linux/man-pages/man5/sshd_config.5.html",
    "mount(8)": "https://man7.org/linux/man-pages/man8/mount.8.html",
    "usermod": "https://man7.org/linux/man-pages/man8/usermod.8.html",
    "modprobe": "https://man7.org/linux/man-pages/man8/modprobe.8.html",
    "udevadm": f"{_SYSTEMD}/udevadm.html",
    "systemd.exec(5)": f"{_SYSTEMD}/systemd.exec.html",
    "systemd.kill(5)": f"{_SYSTEMD}/systemd.kill.html",
    "systemd.mount(5)": f"{_SYSTEMD}/systemd.mount.html",
    "systemd.resource-control(5)": f"{_SYSTEMD}/systemd.resource-control.html",
    "systemd.syntax(7)": f"{_SYSTEMD}/systemd.syntax.html",
    "readlink": f"{_MAN1}/readlink.1.html",
    "man": f"{_MAN1}/man.1.html",
    "sha256sum": f"{_MAN1}/sha256sum.1.html",
    "virsh": "https://www.libvirt.org/manpages/virsh.html",
    "virt-install": "https://github.com/virt-manager/virt-manager/blob/main/man/virt-install.rst",
    "make": "https://www.gnu.org/software/make/manual/make.html",
    "socat": "http://www.dest-unreach.org/socat/doc/socat.html",
    "jq": "https://jqlang.org/manual/",
    "curl": "https://curl.se/docs/manpage.html",
    "cloud-localds": "https://manpages.debian.org/unstable/cloud-image-utils/cloud-localds.1.en.html",
    "qemu-system-x86_64(1)": "https://www.qemu.org/docs/master/system/invocation.html",
    "qemu-system(1)": "https://www.qemu.org/docs/master/system/invocation.html",
    "qemu-guest.nix": "https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/profiles/qemu-guest.nix",
    "nix build": f"{_NIX}/nix3-build",
    "nix develop": f"{_NIX}/nix3-develop",
    "nix flake init": f"{_NIX}/nix3-flake-init",
    "nix profile add": f"{_NIX}/nix3-profile-add",
    "nix profile upgrade": f"{_NIX}/nix3-profile-upgrade",
}


def _title_target(text):
    m = re.fullmatch(r"(.+?)\s*<([^<>]+)>", text, re.DOTALL)
    return (m.group(1), m.group(2)) if m else (text, text)


def _cmd_role(name, rawtext, text, lineno, inliner, options=None, content=None):
    title, target = _title_target(text)
    uri = inliner.document.settings.env.config.cmd_links.get(target)
    code = nodes.literal(rawtext, title)
    if uri is None:
        msg = inliner.reporter.error(
            f"cmd: no link registered for {target!r} (add it to cmd_links in "
            f"docs/conf.py)",
            line=lineno,
        )
        return [code], [msg]
    return [nodes.reference(rawtext, "", code, refuri=uri)], []


# ":gh:`<owner/repo/path>`" links a page on GitHub and ":qemu:`<path>`" one
# of QEMU's GitLab project, such as "blob/master/<file>" or
# "commit/<hash>"; each renders as a literal like ":cmd:" and takes the same
# "title <target>" form.
_LINKS = {
    "gh": "https://github.com/{}",
    "qemu": "https://gitlab.com/qemu-project/qemu/-/{}",
}


def _link_role(name, rawtext, text, lineno, inliner, options=None, content=None):
    title, target = _title_target(text)
    code = nodes.literal(rawtext, title)
    return [nodes.reference(rawtext, "", code, refuri=_LINKS[name].format(target))], []


# ":key:`SshPrivateKey=`" links a key or section of the .qemu format to where
# qemu-systemd.unit(5) defines it, rendered as a literal like ":cmd:". The
# anchors are the ids mandoc gives the man page's section headings.
_KEYS = {
    ".qemu": "FILE_FORMAT",
    "[VM]": "The__VM__section",
    "QemuBinary=": "The__VM__section",
    "VirtiofsdBinary=": "The__VM__section",
    "SshPrivateKey=": "The__VM__section",
    "[QEMU]": "The__QEMU__section",
    "[virtiofsd]": "The__virtiofsd__section",
    "qemu-system@.service": "GENERATED_UNITS",
}


def _key_role(name, rawtext, text, lineno, inliner, options=None, content=None):
    title, target = _title_target(text)
    code = nodes.literal(rawtext, title)
    anchor = _KEYS.get(target)
    if anchor is None:
        msg = inliner.reporter.error(
            f"key: no section registered for {target!r} (add it to _KEYS in "
            f"docs/conf.py)",
            line=lineno,
        )
        return [code], [msg]
    docname = inliner.document.settings.env.docname
    page = os.path.relpath("man/qemu-systemd.unit.5", os.path.dirname(docname) or ".")
    return [nodes.reference(rawtext, "", code, refuri=f"{page}.html#{anchor}")], []


# The man pages stay in scdoc under man/, where the package builds them. Each
# docs/man/ page includes its man page as HTML that scdoc and mandoc render
# here, so the web page and the installed page come from one source. mandoc
# heads a section with <h1> and a subsection with <h2>; both move down a
# level so that neither competes with the page title. In a literal block,
# mandoc breaks before a line that starts with a space, which a <pre>
# already does, so its <br/> would show as an empty line. A reference such
# as "systemctl(1)" links to the rendered page when it is one of these, and
# otherwise to the URL cmd_links gives "name(N)" or "name".
_DOCS = os.path.dirname(os.path.abspath(__file__))
_MAN = os.path.join(os.path.dirname(_DOCS), "man")
_GENERATED = os.path.join(_DOCS, "_generated")


def _man_reference(links, own, m):
    name, section = m.group(1), m.group(2)
    ref = f"{name}({section})"
    if f"{name}.{section}" in own:
        uri = f"{name}.{section}.html"
    else:
        uri = links.get(ref) or links.get(name)
    if uri is None:
        logger.warning(
            "man: no link registered for %r (add it to cmd_links in docs/conf.py)",
            ref,
        )
        return m.group(0)
    return f'<a class="reference external" href="{uri}"><b>{ref}</b></a>'


def _render_man_pages(app):
    os.makedirs(_GENERATED, exist_ok=True)
    pages = sorted(p for p in os.listdir(_MAN) if p.endswith(".scd"))
    own = {p[: -len(".scd")] for p in pages}
    for page in pages:
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
        html = re.sub(r"<(/?)h2\b", r"<\1h3", html)
        html = re.sub(r"<(/?)h1\b", r"<\1h2", html)
        html = re.sub(
            r"<pre>.*?</pre>",
            lambda m: m.group(0).replace("<br/>\n", ""),
            html,
            flags=re.DOTALL,
        )
        html = re.sub(
            r"<b>([^<]+)</b>\(([0-9])\)",
            lambda m: _man_reference(app.config.cmd_links, own, m),
            html,
        )
        with open(os.path.join(_GENERATED, page[: -len(".scd")] + ".html"), "w") as out:
            out.write(html)


def setup(app):
    app.add_config_value("cmd_links", {}, "env")
    app.add_role("cmd", _cmd_role)
    app.add_role("key", _key_role)
    for name in _LINKS:
        app.add_role(name, _link_role)
    app.connect("builder-inited", _render_man_pages)
    return {"parallel_read_safe": True, "parallel_write_safe": True}
