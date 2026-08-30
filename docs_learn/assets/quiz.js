document.querySelectorAll("[data-quiz]").forEach((quiz) => {
  const feedback = quiz.querySelector("[data-feedback]");
  const explanation = quiz.dataset.explanation ?? "";

  quiz.querySelectorAll("button[data-answer]").forEach((button) => {
    button.addEventListener("click", () => {
      quiz.querySelectorAll("button[data-answer]").forEach((candidate) => {
        candidate.classList.remove("correct", "incorrect");
      });

      const correct = button.dataset.answer === "correct";
      button.classList.add(correct ? "correct" : "incorrect");
      feedback.textContent = correct
        ? `正确。${explanation}`
        : `还差一步。${explanation}`;
    });
  });
});
