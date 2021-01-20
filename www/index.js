// Some lovely global variables
let canv, ctx;

function onWindowResize() {
    console.log(window.innerWidth, window.innerHeight);
}

function start() {
    canv = document.getElementById("comp-canvas");
    ctx = canv.getContext("2d");
    
    // Correctly handle window resizing
    window.addEventListener('resize', onWindowResize);
    onWindowResize();

    ctx.fillRect(10, 10, 30, 30);

    console.log(reverse("Hello"));
}
