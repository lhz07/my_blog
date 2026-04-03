const popover = document.getElementById("tagPopover");
const toggleBtn = document.getElementById("toggleTagBtn");
const tagButtons = document.querySelectorAll("#tagBar button");
const tagSearchInput = document.getElementById("tagSearch");
const tagSidebar = document.getElementById("tagSidebar");
const form = document.getElementById("searchForm");
const clearBtn = document.getElementById("clearBtn");
form.addEventListener("submit", (e) => {
  // handle tags in form submission
  // clear tags
  form.querySelector("input[name='tag']")?.remove();
  // add selected tags to form
  const selectedTags = [...document.querySelectorAll("#tagBar .active")].map(
    (b) => b.dataset.tag,
  );
  if (selectedTags.length > 0) {
    const tag_str = selectedTags.join(",");
    const input = document.createElement("input");
    input.type = "hidden";
    input.name = "tag";
    input.value = tag_str;
    form.appendChild(input);
  }
  // handle empty search query
  const input_q = form.querySelector("input[id='searchInput']");
  // prevent empty search
  if (input_q.value.trim() === "") {
    if (selectedTags.length === 0) {
      e.preventDefault();
    } else {
      // allow search if tags are selected
      // delete the empty q input to avoid sending it to server
      input_q.removeAttribute("name");
    }
  } else if (input_q.getAttribute("name") === null) {
    // restore the attribute
    input_q.setAttribute("name", "q");
  }
});

toggleBtn.addEventListener("click", () => {
  // tagSidebar.classList.toggle("hidden");
  popover.classList.toggle("scale-50");
  popover.classList.toggle("opacity-0");
  popover.classList.toggle("pointer-events-none");
  popover.classList.toggle("-translate-y-20");
  popover.classList.toggle("translate-x-12");
});
tagButtons.forEach((btn) => {
  btn.addEventListener("click", () => {
    btn.classList.toggle("active");
  });
});
tagSearchInput.addEventListener("input", () => {
  const val = tagSearchInput.value.toLowerCase();
  tagButtons.forEach((btn) => {
    btn.style.display = btn.dataset.tag.toLowerCase().includes(val)
      ? "inline-flex"
      : "none";
  });
});

clearBtn.addEventListener("click", () => {
  tagButtons.forEach((btn) => {
    btn.classList.remove("active");
  });
});

// // Hide popover when clicking outside
// document.addEventListener("click", (e) => {
//   if (!popover.contains(e.target) && !btn.contains(e.target)) {
//     popover.classList.add("hidden");
//   }
// });
