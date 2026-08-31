document.querySelectorAll("[data-quiz]").forEach((quiz) => {
  const answer = quiz.getAttribute("data-answer");
  const explanation = quiz.getAttribute("data-explanation") ?? "";
  const feedback = quiz.querySelector("[data-feedback]");

  quiz.querySelectorAll("button[data-choice]").forEach((button) => {
    button.addEventListener("click", () => {
      quiz.querySelectorAll("button[data-choice]").forEach((candidate) => {
        candidate.classList.remove("correct", "wrong");
      });
      const correct = button.getAttribute("data-choice") === answer;
      button.classList.add(correct ? "correct" : "wrong");
      if (feedback) {
        feedback.textContent = `${correct ? "答对了。" : "再想想。"}${explanation}`;
      }
    });
  });
});
