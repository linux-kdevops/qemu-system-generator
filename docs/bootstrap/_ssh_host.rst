.. SPDX-License-Identifier: copyleft-next-0.3.1

The block dials the guest's context id through
:cmd:`systemd-ssh-proxy(1)`, which systemd installs in its private library
directory, ``/usr/lib/systemd/`` on most distributions; ``systemd-path
system-library-private`` prints the prefix. ``ProxyUseFdpass`` has the
proxy hand ssh the connected socket, and ``IdentitiesOnly`` keeps an
agent's other keys from being offered first. A guest makes new host keys
on a fresh root file system, so ``UserKnownHostsFile /dev/null`` keeps
them out of your ``known_hosts``, and each connection prints that it
added one.
