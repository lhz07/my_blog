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
  button.addEventListener("click", () => {
    if (code) {
      navigator.clipboard.writeText(code.innerText).then(() => {
        const old = button.innerText;
        button.innerText = "Copied!";
        setTimeout(() => (button.innerText = old), 2000);
      });
    }
  });
  pre.appendChild(button);
  hljs.highlightElement(code);
});
