document.documentElement.setAttribute(
  "data-theme",
  localStorage.getItem("theme") === "dark" ? "dark" : "light"
);
