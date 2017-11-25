window.onload = function() {
    let parallax = document.getElementsByClassName("parallax");
    document.addEventListener("scroll", function(e) {
        for (elem of parallax) {
            elem.style.backgroundPosition = "0 " + (447 - window.scrollY*0.1 - 447) + "px";
        }
    });
};
