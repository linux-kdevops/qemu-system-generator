.. SPDX-License-Identifier: copyleft-next-0.3.1

``systemctl --user stop`` would press the power button and wait two
minutes for a guest that cannot answer it. Kill it instead; QEMU exits on
``SIGTERM``:

.. code-block:: console
   :class: cmd-host

   $ systemctl --user kill qemu-system@bootstrap.service
