function parseRunnableDirective(codeText) {
  const lines = codeText.split("\n");
  if (lines.length === 0) return null;

  const firstLine = lines[0].trim();
  // first comment
  if (!firstLine.startsWith("//")) return null;
  const tokens = firstLine.slice(2).trim().split(/\s+/);
  // include runnable
  if (tokens.length === 0 || tokens[0] !== "runnable") {
    return null;
  }
  // default
  let version = "stable";
  let optimize = "0";
  let edition = "2024";
  // valid value
  const validVersions = new Set(["stable", "beta", "nightly"]);
  const validOptimize = {
    debug: "0",
    release: "3",
  };
  for (let i = 1; i < tokens.length; i++) {
    const t = tokens[i];
    if (validVersions.has(t)) {
      version = t;
      continue;
    }
    if (t in validOptimize) {
      optimize = validOptimize[t];
      continue;
    }
    if (/^20\d{2}$/.test(t)) {
      edition = t;
      continue;
    }
  }

  return {
    version,
    optimize,
    edition,
  };
}

async function runRust(code, config, btn, signal) {
  let outputBox = code.parentNode.parentNode.querySelector(".output-box");

  if (!outputBox) {
    outputBox = document.createElement("div");
    outputBox.className = "output-box";
    outputBox.innerHTML = `
          <div class="output-content"></div>
      `;

    code.parentNode.parentNode.appendChild(outputBox);
  }

  const content = outputBox.querySelector(".output-content");
  content.textContent = "Running...";
  content.style.fontStyle = "italic";
  outputBox.classList.remove("hidden");
  const allCodeClean = Array.from(code.querySelectorAll("div"))
    .map((div) => div.textContent)
    .join("\n");
  try {
    const response = await fetch("https://play.rust-lang.org/evaluate.json", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        version: config.version,
        optimize: config.optimize,
        edition: config.edition,
        code: allCodeClean,
      }),
      signal,
    });

    const result = await response.json();

    let text = "";
    if (result.error && !response.ok) {
      throw new Error(result.error);
    } else if (result.result) {
      text = result.result;
    }
    if (text.length !== 0) {
      content.textContent = text;
      content.style.fontStyle = "normal";
    } else {
      content.textContent = "No output";
      content.style.fontStyle = "italic";
    }
  } catch (e) {
    if (e.name === "AbortError") {
      return;
    }
    content.textContent = "request error: " + e.message;
  }
}

function parseAnchor(code, buttonsGroup, comment_sign) {
  const child = Array.from(code.children);
  let isAnchorContent = false;
  let hasAnchors = false;
  child.forEach((line) => {
    const text = line.textContent.trim();
    if (!text.startsWith(comment_sign) && !isAnchorContent) {
      return;
    }
    const maybeAnchor = text.slice(2).trim();
    if (maybeAnchor === "ANCHOR") {
      isAnchorContent = true;
      line.remove();
      hasAnchors = true;
    } else if (maybeAnchor === "ANCHOR_END") {
      isAnchorContent = false;
      line.remove();
    } else if (isAnchorContent) {
      line.setAttribute("data-anchor", "true");
    }
  });
  if (!hasAnchors) {
    return;
  }
  const visibleBtn = document.createElement("button");
  visibleBtn.className = "visible-btn";
  const visibleOff = `<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor"><path d="M607-627q29 29 42.5 66t9.5 76q0 15-11 25.5T622-449q-15 0-25.5-10.5T586-485q5-26-3-50t-25-41q-17-17-41-26t-51-4q-15 0-25.5-11T430-643q0-15 10.5-25.5T466-679q38-4 75 9.5t66 42.5Zm-127-93q-19 0-37 1.5t-36 5.5q-17 3-30.5-5T358-742q-5-16 3.5-31t24.5-18q23-5 46.5-7t47.5-2q137 0 250.5 72T904-534q4 8 6 16.5t2 17.5q0 9-1.5 17.5T905-466q-18 40-44.5 75T802-327q-12 11-28 9t-26-16q-10-14-8.5-30.5T753-392q24-23 44-50t35-58q-50-101-144.5-160.5T480-720Zm0 520q-134 0-245-72.5T60-463q-5-8-7.5-17.5T50-500q0-10 2-19t7-18q20-40 46.5-76.5T166-680l-83-84q-11-12-10.5-28.5T84-820q11-11 28-11t28 11l680 680q11 11 11.5 27.5T820-84q-11 11-28 11t-28-11L624-222q-35 11-71 16.5t-73 5.5ZM222-624q-29 26-53 57t-41 67q50 101 144.5 160.5T480-280q20 0 39-2.5t39-5.5l-36-38q-11 3-21 4.5t-21 1.5q-75 0-127.5-52.5T300-500q0-11 1.5-21t4.5-21l-84-82Zm319 93Zm-151 75Z"/></svg>`;
  const visibleOn = `<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor"><path d="M607.5-372.5Q660-425 660-500t-52.5-127.5Q555-680 480-680t-127.5 52.5Q300-575 300-500t52.5 127.5Q405-320 480-320t127.5-52.5Zm-204-51Q372-455 372-500t31.5-76.5Q435-608 480-608t76.5 31.5Q588-545 588-500t-31.5 76.5Q525-392 480-392t-76.5-31.5ZM235.5-272Q125-344 61-462q-5-9-7.5-18.5T51-500q0-10 2.5-19.5T61-538q64-118 174.5-190T480-800q134 0 244.5 72T899-538q5 9 7.5 18.5T909-500q0 10-2.5 19.5T899-462q-64 118-174.5 190T480-200q-134 0-244.5-72ZM480-500Zm207.5 160.5Q782-399 832-500q-50-101-144.5-160.5T480-720q-113 0-207.5 59.5T128-500q50 101 144.5 160.5T480-280q113 0 207.5-59.5Z"/></svg>`;
  visibleBtn.innerHTML = visibleOn;
  let showingOnlyAnchors = true;
  code.classList.add("show-snippets-only");
  visibleBtn.onclick = () => {
    showingOnlyAnchors = !showingOnlyAnchors;
    if (showingOnlyAnchors) {
      visibleBtn.innerHTML = visibleOn;
      code.classList.add("show-snippets-only");
    } else {
      visibleBtn.innerHTML = visibleOff;
      code.classList.remove("show-snippets-only");
    }
  };
  buttonsGroup.prepend(visibleBtn);
}

document.querySelectorAll("pre").forEach((pre) => {
  const wrapper = document.createElement("div");
  wrapper.className = "code-group group";
  pre.parentNode.insertBefore(wrapper, pre);
  wrapper.appendChild(pre);
  const code = pre.querySelector("code");
  // highlight code
  hljs.highlightElement(code);
  // convert to divs
  const html = code.innerHTML.trimEnd();
  const lines = html.split(/\r?\n/);
  code.innerHTML = lines.map((line) => `<div>${line || " "}</div>`).join("");
  // get language name
  let code_name = "text";
  code.classList.forEach((cls) => {
    if (cls.startsWith("language-")) {
      code_name = cls.replace("language-", "");
      return;
    }
  });
  // create buttons
  const buttonsGroup = document.createElement("div");
  buttonsGroup.className = "btns-group";
  // create copy button
  const button = document.createElement("button");
  button.innerText = code_name;
  button.className = "copy-btn";
  const old = button.innerText;
  button.addEventListener("click", () => {
    if (code) {
      navigator.clipboard.writeText(code.innerText).then(() => {
        button.innerText = "Copied!";
        setTimeout(() => (button.innerText = old), 2000);
      });
    }
  });
  buttonsGroup.appendChild(button);
  code_name = code_name.toLowerCase();
  const codeStartIcon = `<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor"><path d="M320-273v-414q0-17 12-28.5t28-11.5q5 0 10.5 1.5T381-721l326 207q9 6 13.5 15t4.5 19q0 10-4.5 19T707-446L381-239q-5 3-10.5 4.5T360-233q-16 0-28-11.5T320-273Zm80-207Zm0 134 210-134-210-134v268Z"/></svg>`;
  const codeStopIcon = `<svg xmlns="http://www.w3.org/2000/svg" height="16px" viewBox="0 -960 960 960" width="16px" fill="currentColor"><path d="M240-320v-320q0-33 23.5-56.5T320-720h320q33 0 56.5 23.5T720-640v320q0 33-23.5 56.5T640-240H320q-33 0-56.5-23.5T240-320Zm80 0h320v-320H320v320Zm160-160Z"/></svg>`;
  if (code_name === "rust") {
    const config = parseRunnableDirective(code.innerText);
    if (config) {
      code.removeChild(code.firstElementChild);
      const runButton = document.createElement("button");
      runButton.innerHTML = codeStartIcon;
      runButton.className = "run-btn";
      buttonsGroup.prepend(runButton);
      let controller = null;
      runButton.addEventListener("click", () => {
        if (code) {
          if (!controller) {
            controller = new AbortController();
            runButton.innerHTML = codeStopIcon;
            runRust(code, config, runButton, controller.signal).then(() => {
              controller = null;
              runButton.innerHTML = codeStartIcon;
            });
          } else {
            controller.abort();
            const outputBox =
              code.parentNode.parentNode.querySelector(".output-box");
            if (outputBox) {
              outputBox.classList.add("hidden");
            }
          }
        }
      });
    }
    parseAnchor(code, buttonsGroup, "//");
  } else if (code_name == "c" || code_name == "cpp") {
    parseAnchor(code, buttonsGroup, "//");
  } else if (code_name == "py" || code_name == "python") {
    parseAnchor(code, buttonsGroup, "#");
  }
  pre.appendChild(buttonsGroup);
});

document.querySelectorAll(".markdown-body a[href]").forEach((a) => {
  const url = a.getAttribute("href");
  if (url.startsWith("http://") || url.startsWith("https://")) {
    a.setAttribute("target", "_blank");
  }
});

const tocToggleButton = document.getElementById("toc-toggle-btn");
// Generate TOC starting from <h2>, with smooth scrolling and active highlighting
function generateTOC(containerSelector = "#toc") {
  const article_header = document.getElementById("article-header");
  const article = document.getElementById("article-content");
  const main_offset =
    article_header.offsetHeight + article.offsetTop + article.offsetHeight - 65;
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
    tocContainer.classList.remove("hidden");
    tocToggleButton.classList.remove("hidden");
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
            top: target.offsetTop + article_header.offsetHeight + 20,
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
  const container = document.getElementById("toc-container");

  function updateActiveLink() {
    const scrollY = window.scrollY;
    if (scrollY > main_offset) {
      tocToggleButton.classList.add("opacity-0");
      if (!container.classList.contains("opacity-0")) {
        tocToggleButton.click();
      }
    } else {
      tocToggleButton.classList.remove("opacity-0");
    }
    let currentId = null;
    for (let i = 0; i < headingOffsets.length; i++) {
      if (
        scrollY >=
        headingOffsets[i].offset + article_header.offsetHeight + 20
      ) {
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

  tocToggleButton.addEventListener("click", () => {
    container.classList.toggle("scale-50");
    container.classList.toggle("opacity-0");
    container.classList.toggle("-translate-y-32");
    container.classList.toggle("translate-x-12");
    container.classList.toggle("pointer-events-none");
  });

  // end
}
generateTOC();
