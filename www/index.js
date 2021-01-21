/* ===== CONSTS ===== */

// The 'Device Pixel Ratio'.  For screens with lots of pixels, `1px` might correspond to multiple
// real life pixels - so dpr provides that scale-up
const dpr = window.devicePixelRatio || 1;

/* ===== GLOBAL VARIABLES ===== */

// Variables set in the `start()` function
let canv, ctx;

function onWindowResize() {
    // Set the canvas size according to its new on-screen size
    var rect = canv.getBoundingClientRect();
    canv.width = rect.width * dpr;
    canv.height = rect.height * dpr;

    // Request a frame to be drawn
    draw();
}

function draw() {
    // Clear the screen and correct for HDPI displays
    ctx.save();
    ctx.clearRect(0, 0, canv.width, canv.height);
    ctx.scale(dpr, dpr);

    ctx.beginPath();
    ctx.arc(100, 100, 80, 0, Math.PI * 2);
    ctx.stroke();

    // Reset the canvas' transform matrix so that the next frame is rendered correctly
    ctx.restore();
}

function start() {
    canv = document.getElementById("comp-canvas");
    ctx = canv.getContext("2d");
    
    // Correctly handle window resizing
    window.addEventListener('resize', onWindowResize);
    onWindowResize();

    console.log(reverse("Hello"));

    window.requestAnimationFrame(draw);
}
