G1 X50 Y0   ; Trage linia de jos
G1 X50 Y50  ; Trage linia din dreapta
G1 X0 Y50   ; Trage linia de sus
G1 X0 Y0    ; Trage linia din stanga (patratul e inchis)

; --- PRIMA DIAGONALA ---
G1 X50 Y50  ; Trage diagonala de la stanga-jos la dreapta-sus

; --- A DOUA DIAGONALA ---
M5          ; Ridica creionul (Pen UP)
G1 X50 Y0   ; Se muta la coltul din dreapta-jos prin aer
M3          ; Coboara creionul (Pen DOWN)
G1 X0 Y50   ; Trage diagonala de la dreapta-jos la stanga-sus

; --- FINALIZARE ---
M5          ; Ridica creionul (Pen UP)
G1 X0 Y0    ; Se intoarce la punctul de plecare (Home)