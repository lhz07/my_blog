// Modal elements
const toggleBtn = document.getElementById("toggleFormBtn");
const modalOverlay = document.getElementById("modalOverlay");
const closeModalBtn = document.getElementById("closeModalBtn");
const cancelBtn = document.getElementById("cancelFormBtn");
const submitForm = document.getElementById("friendLinkSubmitForm");

// Open modal
toggleBtn.addEventListener("click", () => {
  modalOverlay.classList.remove("opacity-0", "pointer-events-none");
  modalOverlay.classList.add("opacity-100");
  document.body.style.overflow = "hidden"; // Prevent background scrolling
});

// Close modal function
function closeModal() {
  modalOverlay.classList.remove("opacity-100", "h-screen");
  modalOverlay.classList.add("opacity-0", "pointer-events-none");

  document.body.style.overflow = "auto"; // Restore scrolling
  submitForm.reset();
}

// Close modal events
closeModalBtn.addEventListener("click", closeModal);
cancelBtn.addEventListener("click", closeModal);

// Close modal with Escape key
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !modalOverlay.classList.contains("hidden")) {
    closeModal();
  }
});

// Handle form submission
submitForm.addEventListener("submit", async (e) => {
  e.preventDefault();

  const formData = new FormData(submitForm);
  const data = Object.fromEntries(formData.entries());
  const submitButton = e.target.querySelector('button[type="submit"]');
  const originalText = submitButton.textContent;
  try {
    // Show loading state
    submitButton.textContent = "提交中...";
    submitButton.disabled = true;

    // Send request to your backend
    const response = await fetch("/api/friend-link", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(data),
    });

    if (response.ok) {
      alert("友链申请已提交，我会尽快查看（大概）");
      closeModal();
    } else if (
      response.status === 500 &&
      response.headers.get("Content-Type") == "text/plain"
    ) {
      alert(`提交失败: ${await response.text()}\n请通过邮件联系我`);
    }
  } catch (error) {
    alert("提交失败，请稍后重试或通过邮件联系我");
  } finally {
    // Reset button state
    submitButton.textContent = originalText;
    submitButton.disabled = false;
  }
});
