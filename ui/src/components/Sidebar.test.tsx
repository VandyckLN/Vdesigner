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
});
