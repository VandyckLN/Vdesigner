import { render, screen, waitFor, within } from "@testing-library/react";
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

/** The exact rgb() jsdom serialises a #rrggbb value to — the yardstick for
 *  "this chip paints precisely the hex the label reports". */
function hexToRgb(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  return `rgb(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255})`;
}

let aoCorCapturada: ((hex: string) => void) | null = null;
function emitirCorCapturada(hex: string) {
  aoCorCapturada?.(hex);
}

beforeEach(() => {
  vi.restoreAllMocks();
  aoCorCapturada = null;
  vi.spyOn(api, "onColorPicked").mockImplementation((handler) => {
    aoCorCapturada = handler;
    return Promise.resolve(() => {});
  });
  vi.spyOn(api, "startPick").mockResolvedValue({
    origin_x: 0,
    origin_y: 0,
    width: 1920,
    height: 1080,
    png_base64: "",
    monitors: [],
  });
  vi.spyOn(api, "colorVariations").mockResolvedValue(variations);
  vi.spyOn(api, "formatColor").mockResolvedValue("rgb(138, 144, 150)");
  vi.spyOn(api, "loadPalette").mockResolvedValue(null);
  vi.spyOn(api, "savePalette").mockResolvedValue("{}");
});

describe("Colors", () => {
  it("mostra os dez degraus da escala ao digitar uma cor", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");

    const ramp = await screen.findByRole("list", { name: /escala/i });
    // Scoped to the ramp list specifically: the harmony list also renders
    // <li> elements, so an unscoped query would pass even if the ramp itself
    // rendered only half its steps (the harmony's 5 plus a half-rendered
    // ramp's 5 would still clear a >= 10 threshold).
    expect(within(ramp).getAllByRole("listitem")).toHaveLength(10);
    expect(screen.getByText("500")).toBeInTheDocument();
    // A real hex value from the mocked API data, not a placeholder, proves
    // the component actually rendered what `variations.ramp` returned.
    expect(within(ramp).getByText("#8A9096")).toBeInTheDocument();
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
    await userEvent.click(screen.getByRole("button", { name: /^adicionar$/i }));

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
    await userEvent.click(screen.getByRole("button", { name: /^adicionar$/i }));

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

  it("grava a paleta carregada sem exigir nome quando se clica em Gravar", async () => {
    vi.spyOn(api, "loadPalette").mockResolvedValue({
      palette: {
        versao: 1,
        nome: "paleta",
        gerar: ["css"],
        cores: [{ nome: "acento", hex: "#8A9096", rampa: false }],
        degrades: [],
      },
      on_disk: "existing-text",
    });
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: /escolher pasta/i }));

    // Name and hex fields stay empty — "Gravar" must not care.
    await userEvent.click(await screen.findByRole("button", { name: /^gravar$/i }));

    await waitFor(() => {
      expect(api.savePalette).toHaveBeenCalledWith(
        "D:/projeto",
        expect.objectContaining({
          cores: [{ nome: "acento", hex: "#8A9096", rampa: false }],
        }),
        "existing-text",
      );
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("continua validando o nome ao usar Adicionar mesmo com Gravar disponível", async () => {
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: /escolher pasta/i }));
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");
    await userEvent.type(screen.getByLabelText(/nome da cor/i), "Nome Inválido");
    await userEvent.click(await screen.findByRole("button", { name: /^adicionar$/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/minúsculas/i);
    expect(api.savePalette).not.toHaveBeenCalled();
  });

  it("mostra um alerta e não deixa gravar com dados antigos quando a pasta escolhida falha ao carregar", async () => {
    vi.spyOn(api, "loadPalette").mockRejectedValue(new Error("json corrompido"));
    render(<Colors onPickDirectory={async () => "D:/projeto-ruim"} />);
    await userEvent.click(screen.getByRole("button", { name: /escolher pasta/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/não foi possível/i);

    // dir was never committed, so Gravar/Adicionar stay disabled and cannot
    // ship the previous (or empty) snapshot under the failed directory.
    expect(screen.getByRole("button", { name: /^gravar$/i })).toBeDisabled();
    expect(api.savePalette).not.toHaveBeenCalled();
  });

  // jsdom does not composite, so no rendering test can catch a blend mode.
  // This asserts the two properties that make the swatch trustworthy on a
  // real screen: the chip carries the exact hex inline, and nothing on the
  // element that paints the colour blends or mixes it.
  it("pinta cada degrau com o hex exato e sem modo de mistura", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");

    const ramp = await screen.findByRole("list", { name: /escala/i });
    const items = within(ramp).getAllByRole("listitem");
    items.forEach((item, index) => {
      const chip = item.querySelector(".swatch-chip") as HTMLElement;
      expect(chip).not.toBeNull();
      // jsdom's CSSOM re-serialises a hex as rgb(), so compare against the
      // hex's exact rgb() form: any mixing or rounding on the way in would
      // change these numbers.
      expect(chip.style.background).toBe(hexToRgb(variations.ramp[index]));
      expect(chip.style.mixBlendMode).toBe("");
      // The colour lives on the chip, never on the list item that also holds
      // the label — that is what used to composite with the page backdrop.
      expect(item.style.background).toBe("");
      expect(item.style.mixBlendMode).toBe("");
    });
  });

  it("aceita um hex de três dígitos sem cerquilha", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "0f8");

    await waitFor(() => {
      expect(api.colorVariations).toHaveBeenCalledWith("#0f8");
    });
    expect(await screen.findByRole("list", { name: /escala/i })).toBeInTheDocument();
  });

  it("explica por que um hex com tamanho errado foi recusado", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#12345");

    expect(await screen.findByRole("alert")).toHaveTextContent(/três ou seis/i);
    expect(screen.queryByRole("list", { name: /escala/i })).not.toBeInTheDocument();
  });

  it("avisa quando a área de transferência recusa a cópia", async () => {
    vi.mocked(writeText).mockRejectedValueOnce(new Error("clipboard bloqueada"));
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");
    await userEvent.click(await screen.findByRole("button", { name: /copiar/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/não foi possível copiar/i);
  });

  it("avisa quando o motor falha ao formatar a cor para cópia", async () => {
    vi.spyOn(api, "formatColor").mockRejectedValue(new Error("formato desconhecido"));
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");
    await userEvent.click(await screen.findByRole("button", { name: /copiar/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/não foi possível copiar/i);
  });

  it("mostra as cores da paleta carregada com nome e hex em texto", async () => {
    vi.spyOn(api, "loadPalette").mockResolvedValue({
      palette: {
        versao: 1,
        nome: "paleta",
        gerar: ["css"],
        cores: [
          { nome: "acento", hex: "#8A9096", rampa: false },
          { nome: "tinta", hex: "#EDE8DE", rampa: true },
        ],
        degrades: [],
      },
      on_disk: "existing-text",
    });
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: /escolher pasta/i }));

    const list = await screen.findByRole("list", { name: /cores da paleta/i });
    expect(within(list).getAllByRole("listitem")).toHaveLength(2);
    expect(within(list).getByText("acento")).toBeInTheDocument();
    expect(within(list).getByText("#8A9096")).toBeInTheDocument();
    expect(within(list).getByText("tinta")).toBeInTheDocument();
    expect(within(list).getByText("#EDE8DE")).toBeInTheDocument();
    // The remove control names the colour it removes, so it is unambiguous
    // to anyone hearing the buttons out of context.
    expect(
      within(list).getByRole("button", { name: /remover a cor acento/i }),
    ).toBeInTheDocument();
  });

  it("remove uma cor e grava a paleta encurtada", async () => {
    vi.spyOn(api, "loadPalette").mockResolvedValue({
      palette: {
        versao: 1,
        nome: "paleta",
        gerar: ["css"],
        cores: [
          { nome: "acento", hex: "#8A9096", rampa: false },
          { nome: "tinta", hex: "#EDE8DE", rampa: true },
        ],
        degrades: [],
      },
      on_disk: "existing-text",
    });
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: /escolher pasta/i }));
    await userEvent.click(
      await screen.findByRole("button", { name: /remover a cor acento/i }),
    );

    await waitFor(() => {
      expect(api.savePalette).toHaveBeenCalledWith(
        "D:/projeto",
        expect.objectContaining({
          cores: [{ nome: "tinta", hex: "#EDE8DE", rampa: true }],
        }),
        "existing-text",
      );
    });
    const list = await screen.findByRole("list", { name: /cores da paleta/i });
    expect(within(list).queryByText("acento")).not.toBeInTheDocument();
  });

  it("leva a escolha de gerar a escala para o payload gravado", async () => {
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: /escolher pasta/i }));
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");
    await userEvent.type(screen.getByLabelText(/nome da cor/i), "acento");
    await userEvent.click(screen.getByLabelText(/gerar a escala de tons/i));
    await userEvent.click(screen.getByRole("button", { name: /^adicionar$/i }));

    await waitFor(() => {
      expect(api.savePalette).toHaveBeenCalledWith(
        "D:/projeto",
        expect.objectContaining({
          cores: [{ nome: "acento", hex: "#8A9096", rampa: true }],
        }),
        null,
      );
    });
  });

  it("grava rampa falsa quando a escala não é pedida", async () => {
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: /escolher pasta/i }));
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");
    await userEvent.type(screen.getByLabelText(/nome da cor/i), "acento");
    await userEvent.click(screen.getByRole("button", { name: /^adicionar$/i }));

    await waitFor(() => {
      expect(api.savePalette).toHaveBeenCalledWith(
        "D:/projeto",
        expect.objectContaining({
          cores: [{ nome: "acento", hex: "#8A9096", rampa: false }],
        }),
        null,
      );
    });
  });

  it("identifica cada harmonia com um rótulo em português", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.type(screen.getByLabelText(/cor em hex/i), "#8A9096");

    await screen.findByRole("list", { name: /harmonias/i });
    expect(screen.getByText("Complementar")).toBeInTheDocument();
    expect(screen.getByText("Análoga 1")).toBeInTheDocument();
    expect(screen.getByText("Tríade 1")).toBeInTheDocument();
  });

  it("mostra a cor capturada na faixa e a copia para a área de transferência", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    emitirCorCapturada("#3366ff");
    expect(await screen.findByText("#3366ff")).toBeInTheDocument();
    await waitFor(() => expect(writeText).toHaveBeenCalledWith("#3366ff"));
  });

  it("não grava a cor capturada na paleta sozinha", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    emitirCorCapturada("#3366ff");
    await screen.findByText("#3366ff");
    expect(api.savePalette).not.toHaveBeenCalled();
  });

  it("promove uma cor capturada para a paleta com nome", async () => {
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: "Escolher pasta" }));
    emitirCorCapturada("#3366ff");
    await userEvent.click(await screen.findByRole("button", { name: "Usar a cor #3366ff" }));
    await userEvent.type(screen.getByLabelText("Nome da cor"), "marca");
    await userEvent.click(screen.getByRole("button", { name: "Adicionar" }));
    await waitFor(() => expect(api.savePalette).toHaveBeenCalled());
    const [, paleta] = vi.mocked(api.savePalette).mock.calls[0];
    expect(paleta.cores).toContainEqual({ nome: "marca", hex: "#3366ff", rampa: false });
  });

  it("dispara a captura pelo botão", async () => {
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.click(screen.getByRole("button", { name: "Capturar cor" }));
    expect(api.startPick).toHaveBeenCalledTimes(1);
  });

  it("mostra alerta quando a captura falha ao clicar em Capturar cor", async () => {
    vi.spyOn(api, "startPick").mockRejectedValue(new Error("nenhum monitor foi encontrado"));
    render(<Colors onPickDirectory={async () => null} />);
    await userEvent.click(screen.getByRole("button", { name: "Capturar cor" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Não foi possível capturar a tela");
  });

  it("cria um degradê entre duas cores da paleta", async () => {
    vi.spyOn(api, "loadPalette").mockResolvedValue({
      palette: {
        versao: 1,
        nome: "paleta",
        gerar: ["css"],
        cores: [
          { nome: "tinta", hex: "#ede8de", rampa: false },
          { nome: "acento", hex: "#8a9096", rampa: false },
        ],
        degrades: [],
      },
      on_disk: "{}",
    });
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: "Escolher pasta" }));
    await userEvent.type(await screen.findByLabelText("Nome do degradê"), "fundo");
    await userEvent.selectOptions(screen.getByLabelText("De"), "tinta");
    await userEvent.selectOptions(screen.getByLabelText("Para"), "acento");
    await userEvent.click(screen.getByRole("button", { name: "Adicionar degradê" }));
    await waitFor(() => expect(api.savePalette).toHaveBeenCalled());
    const [, paleta] = vi.mocked(api.savePalette).mock.calls.at(-1)!;
    expect(paleta.degrades).toContainEqual({ nome: "fundo", de: "tinta", para: "acento" });
  });

  it("recusa um degradê sem as duas pontas escolhidas", async () => {
    vi.spyOn(api, "loadPalette").mockResolvedValue({
      palette: {
        versao: 1,
        nome: "paleta",
        gerar: ["css"],
        cores: [{ nome: "tinta", hex: "#ede8de", rampa: false }],
        degrades: [],
      },
      on_disk: "{}",
    });
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: "Escolher pasta" }));
    await userEvent.type(await screen.findByLabelText("Nome do degradê"), "fundo");
    await userEvent.click(screen.getByRole("button", { name: "Adicionar degradê" }));
    expect(await screen.findByRole("alert")).toBeInTheDocument();
    expect(api.savePalette).not.toHaveBeenCalled();
  });

  it("copia a regra CSS do degradê para a área de transferência", async () => {
    vi.spyOn(api, "loadPalette").mockResolvedValue({
      palette: {
        versao: 1,
        nome: "paleta",
        gerar: ["css"],
        cores: [
          { nome: "tinta", hex: "#ede8de", rampa: false },
          { nome: "acento", hex: "#8a9096", rampa: false },
        ],
        degrades: [{ nome: "fundo", de: "tinta", para: "acento" }],
      },
      on_disk: "{}",
    });
    vi.spyOn(api, "gradientCss").mockResolvedValue("linear-gradient(90deg, #ede8de, #8a9096)");
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: "Escolher pasta" }));
    await screen.findByText("fundo");
    await userEvent.click(screen.getByRole("button", { name: "Copiar o degradê fundo" }));
    await waitFor(() => {
      expect(writeText).toHaveBeenCalledWith("linear-gradient(90deg, #ede8de, #8a9096)");
    });
  });

  it("remove um degradê da paleta e salva a alteração", async () => {
    vi.spyOn(api, "loadPalette").mockResolvedValue({
      palette: {
        versao: 1,
        nome: "paleta",
        gerar: ["css"],
        cores: [
          { nome: "tinta", hex: "#ede8de", rampa: false },
          { nome: "acento", hex: "#8a9096", rampa: false },
        ],
        degrades: [{ nome: "fundo", de: "tinta", para: "acento" }],
      },
      on_disk: "{}",
    });
    render(<Colors onPickDirectory={async () => "D:/projeto"} />);
    await userEvent.click(screen.getByRole("button", { name: "Escolher pasta" }));
    await screen.findByText("fundo");
    await userEvent.click(screen.getByRole("button", { name: "Remover o degradê fundo" }));
    await waitFor(() => expect(api.savePalette).toHaveBeenCalled());
    const [, paleta] = vi.mocked(api.savePalette).mock.calls.at(-1)!;
    expect(paleta.degrades).toHaveLength(0);
  });
});


