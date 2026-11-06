/**
 * Plays each project card's entrance once when it reaches the viewport.
 * Cards stay visible if scripting is unavailable or reduced motion is enabled.
 * @module cards
 */

if (!matchMedia("(prefers-reduced-motion: reduce)").matches) {
    const observer = new IntersectionObserver(
        /** @param {IntersectionObserverEntry[]} entries */ (entries) => {
            for (const entry of entries) {
                if (!entry.isIntersecting) continue;
                entry.target.classList.add("card_enter");
                observer.unobserve(entry.target);
            }
        },
        { threshold: 0.1 },
    );

    for (const card of document.querySelectorAll(".game_card, .tool_card")) {
        observer.observe(card);
    }
}
