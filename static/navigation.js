const header = document.querySelector(".site_header");

if (header) {
    new ResizeObserver(([entry]) => {
        document.documentElement.style.setProperty(
            "--header-height",
            `${entry.borderBoxSize[0].blockSize}px`,
        );
    }).observe(header);
}
