async function reveal_image(image) {
    try {
        await image.decode();
        image.parentElement.dataset.imageState = "ready";
    } catch {
        image.parentElement.dataset.imageState = "failed";
    }
}

for (const image of document.querySelectorAll(".image_frame > img")) {
    if (image.complete && image.naturalWidth > 0) continue;
    image.parentElement.dataset.imageState = "loading";
    image.addEventListener("load", () => reveal_image(image), { once: true });
    image.addEventListener(
        "error",
        () => {
            image.parentElement.dataset.imageState = "failed";
        },
        { once: true },
    );
    if (image.complete) reveal_image(image);
}
