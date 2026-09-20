import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Sidebar } from "./Sidebar";

describe("Sidebar", () => {
  it("marca a tela atual para leitores de tela", () => {
    render(<Sidebar current="cores" onChange={() => {}} />);
    expect(screen.getByRole("tab", { name: /cores/i })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: /imagem/i })).toHaveAttribute("aria-selected", "false");
  });

  it("avisa quando outra tela é escolhida", async () => {
    const onChange = vi.fn();
    render(<Sidebar current="imagem" onChange={onChange} />);
    await userEvent.click(screen.getByRole("tab", { name: /cores/i }));
    expect(onChange).toHaveBeenCalledWith("cores");
  });

  it("não avisa de novo ao clicar na tela que já está aberta", async () => {
    const onChange = vi.fn();
    render(<Sidebar current="imagem" onChange={onChange} />);
    await userEvent.click(screen.getByRole("tab", { name: /imagem/i }));
    expect(onChange).not.toHaveBeenCalled();
  });

  it("liga cada aba ao seu painel com aria-controls", () => {
    render(<Sidebar current="imagem" onChange={() => {}} />);
    expect(screen.getByRole("tab", { name: /imagem/i })).toHaveAttribute(
      "aria-controls",
      "panel-imagem",
    );
    expect(screen.getByRole("tab", { name: /cores/i })).toHaveAttribute(
      "aria-controls",
      "panel-cores",
    );
  });

  it("só deixa a aba selecionada na ordem de tabulação (roving tabindex)", () => {
    render(<Sidebar current="imagem" onChange={() => {}} />);
    expect(screen.getByRole("tab", { name: /imagem/i })).toHaveAttribute("tabIndex", "0");
    expect(screen.getByRole("tab", { name: /cores/i })).toHaveAttribute("tabIndex", "-1");
  });

  it("move a seleção para a direita com ArrowRight e o foco acompanha", async () => {
    const onChange = vi.fn();
    render(<Sidebar current="imagem" onChange={onChange} />);
    screen.getByRole("tab", { name: /imagem/i }).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(onChange).toHaveBeenCalledWith("cores");
  });

  it("volta ao final com ArrowLeft a partir da primeira aba (wrap)", async () => {
    const onChange = vi.fn();
    render(<Sidebar current="imagem" onChange={onChange} />);
    screen.getByRole("tab", { name: /imagem/i }).focus();
    await userEvent.keyboard("{ArrowLeft}");
    expect(onChange).toHaveBeenCalledWith("sobre");
  });

  it("Home e End selecionam a primeira e a última aba", async () => {
    const onChange = vi.fn();
    render(<Sidebar current="cores" onChange={onChange} />);
    screen.getByRole("tab", { name: /cores/i }).focus();
    await userEvent.keyboard("{Home}");
    expect(onChange).toHaveBeenCalledWith("imagem");

    onChange.mockClear();
    await userEvent.keyboard("{End}");
    expect(onChange).toHaveBeenCalledWith("sobre");
  });

  it("inclui Sobre como terceira aba", () => {
    render(<Sidebar current="imagem" onChange={() => {}} />);
    expect(screen.getAllByRole("tab")).toHaveLength(3);
    expect(screen.getByRole("tab", { name: "Sobre" })).toBeInTheDocument();
  });
});
