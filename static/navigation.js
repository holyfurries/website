/**
 * Keeps the anchor scroll offset aligned with the header as its height changes.
 * @module navigation
 */

const header = document.querySelector(".site_header");

if (header) {
    new ResizeObserver(
        /** @param {ResizeObserverEntry[]} entries */ ([entry]) => {
            document.documentElement.style.setProperty(
                "--header-height",
                `${entry.borderBoxSize[0].blockSize}px`,
            );
        },
    ).observe(header);
}
