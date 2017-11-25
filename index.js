window.onload = function() {
    let parallax = document.getElementById("parallax");
    document.addEventListener("scroll", function(e) {
        parallax.style.backgroundPosition = "0 " + (447 - window.scrollY*1.3 - 447) + "px";
    });
};
