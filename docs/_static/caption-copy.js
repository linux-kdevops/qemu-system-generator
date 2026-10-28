// SPDX-License-Identifier: copyleft-next-0.3.1

// A caption that names a file gets a button that copies the path, beside
// the permalink "#", so the reader can open the file without selecting it.
// It borrows sphinx-copybutton's icons and tooltip, so it looks and
// sits like the button on the code below.
document.addEventListener("DOMContentLoaded", () => {
    for (const caption of document.querySelectorAll(".code-block-caption")) {
        const text = caption.querySelector(".caption-text");
        const path = text ? text.textContent.trim() : "";
        if (!path.startsWith("~/") && !path.startsWith("/")) {
            continue;
        }
        const button = document.createElement("button");
        button.className = "caption-copybtn o-tooltip--left";
        button.type = "button";
        button.dataset.tooltip = "Copy path";
        button.innerHTML = iconCopy;
        button.addEventListener("click", () => {
            navigator.clipboard.writeText(path).then(() => {
                button.innerHTML = iconCheck;
                button.classList.add("success");
                setTimeout(() => {
                    button.innerHTML = iconCopy;
                    button.classList.remove("success");
                }, 2000);
            });
        });
        caption.append(button);
    }
});
