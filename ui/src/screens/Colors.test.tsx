import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { Colors } from "./Colors";
import { api } from "../api";

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: vi.fn() }));
import { writeText } from "@tauri-apps/plugin-clipboard-manager";

const variations = {
  ramp: [
    "#F7F7F8", "#E9EAEB", "#D2D4D6", "#BCBEC1", "#A5A9AD",
    "#8A9096", "#6F757B", "#555A5F", "#3B3F43", "#212427",
  ],
  complementary: "#968A90",
  analogous: ["#8A9690", "#909096"],
  triad: ["#96908A", "#8A9096"],
};

beforeEach(() => {
  vi.restoreAllMocks();
  vi.spyOn(api, "colorVariations").mockResolvedValue(variations);
  vi.spyOn(api, "formatColor").mockResolvedValue("rgb(138, 144, 150)");
  vi.spyOn(api, "loadPalette").mockResolvedValue(null);
  vi.spyOn(api, "savePalette").mockResolvedValue("{}");
});

describe("Colors", () => {
  it("mostra os dez degraus da escala ao digitar uma cor", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");

    await waitFor(() => {
      expect(screen.getByRole("list", { name: /escala/i })).toBeInTheDocument();
    });
    expect(screen.getAllByRole("listitem").length).toBeGreaterThanOrEqual(10);
    expect(screen.getByText("500")).toBeInTheDocument();
  });

  // Named after the visible effect (an alert with the failure), not after the
  // implementation detail of whether the engine was invoked first — the core
  // is the source of truth for hex validity, so the UI always asks it.
  it("mostra um alerta para um hex inválido", async () => {
    vi.spyOn(api, "colorVariations").mockRejectedValue(new Error("cor inválida"));
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#ZZZZZZ");

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent(/inválid/i);
    });
  });

  it("copia a cor no formato escolhido", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");
    await userEvent.selectOptions(screen.getByLabelText(/formato/i), "Rgb");
    await userEvent.click(await screen.findByRole("button", { name: /copiar/i }));

    await waitFor(() => {
      expect(api.formatColor).toHaveBeenCalledWith("#8A9096", "Rgb");
      expect(writeText).toHaveBeenCalledWith("rgb(138, 144, 150)");
    });
  });

  it("não deixa gravar sem uma pasta escolhida", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");
    expect(await screen.findByRole("button", { name: /gravar/i })).toBeDisabled();
  });

  it("recusa um nome de cor que não vira variável CSS", async () => {
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: /escolher pasta/i }));
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");
    await userEvent.type(screen.getByLabelText(/nome da cor/i), "Cor Principal");
    await userEvent.click(screen.getByRole("button", { name: /adicionar/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/minúsculas/i);
    expect(api.savePalette).not.toHaveBeenCalled();
  });

  // Pins the actual persisted result, not merely that the mock was called —
  // a test that only checked the call would still pass if `add` forgot to
  // update local state from the save's response.
  it("acrescenta a cor à paleta e grava no disco com um nome válido", async () => {
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: /escolher pasta/i }));
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");
    await userEvent.type(screen.getByLabelText(/nome da cor/i), "acento");
    await userEvent.click(screen.getByRole("button", { name: /adicionar/i }));

    await waitFor(() => {
      expect(api.savePalette).toHaveBeenCalledWith(
        "D:/projeto",
        expect.objectContaining({
          cores: [{ nome: "acento", hex: "#8A9096", rampa: false }],
        }),
        null,
      );
    });
    // The name field clears and no error is left behind, confirming the
    // component actually consumed the successful response.
    expect(screen.getByLabelText(/nome da cor/i)).toHaveValue("");
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
