document.querySelectorAll("pre").forEach((pre) => {
  const wrapper = document.createElement("div");
  wrapper.className = "code-group group";
  pre.parentNode.insertBefore(wrapper, pre);
  wrapper.appendChild(pre);
  const code = pre.querySelector("code");
  const code_name = code.className.split("-")[1];
  const button = document.createElement("button");
  button.innerText = code_name;
  button.className = "copy-btn";
  hljs.highlightElement(code);
  const old = button.innerText;
  button.addEventListener("click", () => {
    if (code) {
      navigator.clipboard.writeText(code.innerText).then(() => {
        button.innerText = "Copied!";
        setTimeout(() => (button.innerText = old), 2000);
      });
    }
  });
  pre.appendChild(button);
});
document.querySelectorAll(".markdown-body a[href]").forEach((a) => {
  const url = a.getAttribute("href");
  if (url.startsWith("http://") || url.startsWith("https://")) {
    a.setAttribute("target", "_blank");
  }
});
document.querySelectorAll("pre code").forEach((block) => {
  const html = block.innerHTML.trimEnd();
  const lines = html.split(/\r?\n/);
  block.innerHTML = lines.map((line) => `<div>${line || " "}</div>`).join("");
});
// Generate TOC starting from <h2>, with smooth scrolling and active highlighting
function generateTOC(containerSelector = "#toc") {
  // Collect all headings
  const allHeadings = Array.from(
    document.querySelectorAll("h2, h3, h4, h5, h6"),
  );
  if (allHeadings.length === 0) return;

  // Find the smallest heading level (h1=1, h2=2, etc.)
  const minLevel = Math.min(...allHeadings.map((h) => parseInt(h.tagName[1])));
  const headings = allHeadings.filter(
    (h) => parseInt(h.tagName[1]) >= minLevel,
  );

  const tocContainer = document.querySelector(containerSelector);
  const tocRoot = document.createElement("ul");

  // Create counters for up to 6 levels
  const counters = [0, 0, 0, 0, 0];
  let currentListStack = [tocRoot];
  let lastLevel = 1;

  for (const heading of headings) {
    // Normalize heading level relative to the top-level heading
    const level = parseInt(heading.tagName[1]) - minLevel + 1;
    counters[level - 1]++;
    for (let i = level; i < counters.length; i++) counters[i] = 0;

    const numberParts = counters.slice(0, level).filter((n) => n > 0);
    const numbering = numberParts.join(".");

    const text = heading.textContent.trim().replace(/\s+/g, "_");
    const id = `${numbering}_${text}`;
    heading.id = id;

    // Adjust nesting level
    if (level > lastLevel) {
      const newList = document.createElement("ul");
      const lastLi =
        currentListStack[currentListStack.length - 1].lastElementChild;
      if (lastLi) lastLi.appendChild(newList);
      currentListStack.push(newList);
    } else if (level < lastLevel) {
      currentListStack.splice(level - lastLevel);
    }
    lastLevel = level;

    // Create TOC entry
    const li = document.createElement("li");
    const link = document.createElement("a");
    link.href = `#${id}`;
    link.textContent = `${numbering} ${heading.textContent.trim()}`;
    li.appendChild(link);
    currentListStack[currentListStack.length - 1].appendChild(li);
  }

  if (tocContainer) {
    tocContainer.innerHTML = "";
    tocContainer.appendChild(tocRoot);
  }
  // Smooth scrolling
  document
    .querySelectorAll(`${containerSelector} a[href^="#"]`)
    .forEach((a) => {
      a.addEventListener("click", (e) => {
        history.replaceState(null, "", a.getAttribute("href"));
        e.preventDefault();
        const target = document.getElementById(a.getAttribute("href").slice(1));
        if (target) {
          window.scrollTo({
            top: target.offsetTop + 62 - 10,
            behavior: "smooth",
          });
        }
      });
    });

  // Highlight active heading in TOC
  const tocLinks = Array.from(
    document.querySelectorAll(`${containerSelector} a`),
  );
  const headingOffsets = headings.map((h) => ({
    id: h.id,
    offset: h.offsetTop,
  }));
  function updateActiveLink() {
    const scrollY = window.scrollY;
    let currentId = headingOffsets[0].id;
    for (let i = 0; i < headingOffsets.length; i++) {
      if (scrollY >= headingOffsets[i].offset + 62 - 10) {
        currentId = headingOffsets[i].id;
      } else {
        break;
      }
    }

    tocLinks.forEach((link) => {
      link.classList.toggle(
        "toc-active",
        link.getAttribute("href") === `#${currentId}`,
      );
    });
  }

  window.addEventListener("scroll", updateActiveLink);
  updateActiveLink();

  // end
}
generateTOC();
const tocToggleButton = document.getElementById("toc-toggle-btn");
tocToggleButton.addEventListener("click", () => {
  document.getElementById("toc-container").classList.toggle("hidden");
});
